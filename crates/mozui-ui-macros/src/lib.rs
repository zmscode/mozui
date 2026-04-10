use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};

mod derive_into_plot;

/// Input for icon_name! macro: EnumName, "path"
struct IconNameInput {
    enum_name: syn::Ident,
    _comma: syn::Token![,],
    path: syn::LitStr,
}

impl Parse for IconNameInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(IconNameInput {
            enum_name: input.parse()?,
            _comma: input.parse()?,
            path: input.parse()?,
        })
    }
}

#[proc_macro_derive(IntoPlot)]
pub fn derive_into_plot(input: TokenStream) -> TokenStream {
    derive_into_plot::derive_into_plot(input)
}

/// Convert an SVG filename to PascalCase identifier.
///
/// Strips `.svg` extension, splits on separators (`-`, `_`, `.`),
/// and capitalizes each word following Rust naming conventions.
fn pascal_case(filename: &str) -> String {
    filename
        .strip_suffix(".svg")
        .unwrap_or(filename)
        .split(|c: char| c == '-' || c == '_' || c == '.')
        .filter(|part| !part.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) if first.is_ascii_digit() => word.to_string(),
                Some(first) => {
                    let mut result = String::with_capacity(word.len());
                    result.extend(first.to_uppercase());
                    result.push_str(&chars.as_str().to_lowercase());
                    result
                }
            }
        })
        .collect()
}

/// Known weight directory names, in enum order.
const WEIGHTS: &[&str] = &["thin", "light", "regular", "bold", "fill", "duotone"];

/// Generate a Phosphor icon enum with weight support and embedded SVG data.
///
/// Scans subdirectories (thin/, light/, regular/, bold/, fill/, duotone/) under
/// the given path. Generates an `IconName` enum from the union of SVG filenames
/// and an `IconWeight` enum. Each variant embeds its SVG data via `include_bytes!`.
///
/// # Example
///
/// ```ignore
/// icon_named!(IconName, "icons");
/// ```
#[proc_macro]
pub fn icon_named(input: TokenStream) -> TokenStream {
    let IconNameInput {
        enum_name, path, ..
    } = syn::parse_macro_input!(input as IconNameInput);

    let relative_path = path.value();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let icons_dir = std::path::Path::new(&manifest_dir).join(&relative_path);

    // Collect all unique icon names across all weight directories
    let mut all_icons: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    // Track which weights exist
    let mut available_weights: Vec<&str> = Vec::new();

    for &weight in WEIGHTS {
        let weight_dir = icons_dir.join(weight);
        if weight_dir.is_dir() {
            available_weights.push(weight);
            if let Ok(dir) = std::fs::read_dir(&weight_dir) {
                for entry in dir {
                    let entry = entry.expect("failed to read directory entry");
                    let filename = entry.file_name().to_string_lossy().to_string();
                    if filename.ends_with(".svg") {
                        let name = filename.strip_suffix(".svg").unwrap().to_string();
                        all_icons.insert(name);
                    }
                }
            }
        }
    }

    if available_weights.is_empty() {
        panic!(
            "icon_named!: no weight directories found in '{}'",
            icons_dir.display()
        );
    }

    // Build enum variants
    let entries: Vec<(String, String)> = all_icons
        .iter()
        .map(|name| (pascal_case(&format!("{}.svg", name)), name.clone()))
        .collect();

    let variants: Vec<proc_macro2::Ident> = entries
        .iter()
        .map(|(pascal, _)| proc_macro2::Ident::new(pascal, proc_macro2::Span::call_site()))
        .collect();

    // Build weight enum variants
    let weight_variants: Vec<proc_macro2::Ident> = available_weights
        .iter()
        .map(|w| {
            let pascal = pascal_case(&format!("{}.svg", w));
            proc_macro2::Ident::new(&pascal, proc_macro2::Span::call_site())
        })
        .collect();

    let has_regular = available_weights.contains(&"regular");
    let default_weight = if has_regular {
        quote! {
            impl Default for IconWeight {
                fn default() -> Self {
                    Self::Regular
                }
            }
        }
    } else {
        let first = &weight_variants[0];
        quote! {
            impl Default for IconWeight {
                fn default() -> Self {
                    Self::#first
                }
            }
        }
    };

    // Build the svg_data match arms for each weight
    let weight_match_arms: Vec<proc_macro2::TokenStream> = available_weights
        .iter()
        .zip(weight_variants.iter())
        .map(|(weight, weight_variant)| {
            let weight_dir = icons_dir.join(weight);

            let arms: Vec<proc_macro2::TokenStream> = entries
                .iter()
                .zip(variants.iter())
                .map(|((_, filename), variant)| {
                    let svg_path = weight_dir.join(format!("{}.svg", filename));
                    let svg_path_str = svg_path.to_string_lossy().to_string();

                    if svg_path.exists() {
                        // Use the absolute path for include_bytes
                        let lit = syn::LitStr::new(&svg_path_str, proc_macro2::Span::call_site());
                        quote! {
                            (Self::#variant, IconWeight::#weight_variant) => {
                                include_bytes!(#lit)
                            }
                        }
                    } else {
                        // Icon doesn't exist at this weight — fall back to regular if possible
                        let regular_path = icons_dir.join("regular").join(format!("{}.svg", filename));
                        if regular_path.exists() {
                            let lit = syn::LitStr::new(
                                &regular_path.to_string_lossy(),
                                proc_macro2::Span::call_site(),
                            );
                            quote! {
                                (Self::#variant, IconWeight::#weight_variant) => {
                                    include_bytes!(#lit)
                                }
                            }
                        } else {
                            // Find any weight that has this icon
                            let mut fallback: Option<String> = None;
                            for &w in WEIGHTS {
                                let p = icons_dir.join(w).join(format!("{}.svg", filename));
                                if p.exists() {
                                    fallback = Some(p.to_string_lossy().to_string());
                                    break;
                                }
                            }
                            let lit = syn::LitStr::new(
                                &fallback.unwrap_or_else(|| panic!("No SVG found for icon '{}' in any weight", filename)),
                                proc_macro2::Span::call_site(),
                            );
                            quote! {
                                (Self::#variant, IconWeight::#weight_variant) => {
                                    include_bytes!(#lit)
                                }
                            }
                        }
                    }
                })
                .collect();

            quote! { #(#arms,)* }
        })
        .collect();

    // Build the cache key match arms (weight_prefix/filename for atlas caching)
    let weight_prefix_strs: Vec<String> = available_weights.iter().map(|w| w.to_string()).collect();
    let key_match_arms: Vec<proc_macro2::TokenStream> = available_weights
        .iter()
        .zip(weight_variants.iter())
        .enumerate()
        .map(|(i, (_, weight_variant))| {
            let prefix = &weight_prefix_strs[i];
            let arms: Vec<proc_macro2::TokenStream> = entries
                .iter()
                .zip(variants.iter())
                .map(|((_, filename), variant)| {
                    let key = format!("icons/{}/{}.svg", prefix, filename);
                    quote! {
                        (Self::#variant, IconWeight::#weight_variant) => #key
                    }
                })
                .collect();
            quote! { #(#arms,)* }
        })
        .collect();

    let expanded = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum IconWeight {
            #(#weight_variants,)*
        }

        #default_weight

        #[derive(IntoElement, Clone, Copy)]
        pub enum #enum_name {
            #(#variants,)*
        }

        impl IconNamed for #enum_name {
            fn path(self) -> SharedString {
                self.cache_key(IconWeight::default()).into()
            }

            fn svg_data(self, weight: IconWeight) -> &'static [u8] {
                match (self, weight) {
                    #(#weight_match_arms)*
                }
            }

            fn cache_key(self, weight: IconWeight) -> &'static str {
                match (self, weight) {
                    #(#key_match_arms)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}
