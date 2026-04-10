use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn derive_into_plot(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let type_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics mozui::IntoElement for #type_name #type_generics #where_clause {
            type Element = Self;

            fn into_element(self) -> Self::Element {
                self
            }
        }

        impl #impl_generics mozui::Element for #type_name #type_generics #where_clause {
            type RequestLayoutState = ();
            type PrepaintState = ();

            fn id(&self) -> Option<mozui::ElementId> {
                None
            }

            fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
                None
            }

            fn request_layout(
                &mut self,
                _: Option<&mozui::GlobalElementId>,
                _: Option<&mozui::InspectorElementId>,
                window: &mut mozui::Window,
                cx: &mut mozui::App,
            ) -> (mozui::LayoutId, Self::RequestLayoutState) {
                let style = mozui::Style {
                    size: mozui::Size::full(),
                    ..Default::default()
                };

                (window.request_layout(style, None, cx), ())
            }

            fn prepaint(
                &mut self,
                _: Option<&mozui::GlobalElementId>,
                _: Option<&mozui::InspectorElementId>,
                _: mozui::Bounds<mozui::Pixels>,
                _: &mut Self::RequestLayoutState,
                _: &mut mozui::Window,
                _: &mut mozui::App,
            ) -> Self::PrepaintState {
            }

            fn paint(
                &mut self,
                _: Option<&mozui::GlobalElementId>,
                _: Option<&mozui::InspectorElementId>,
                bounds: mozui::Bounds<mozui::Pixels>,
                _: &mut Self::RequestLayoutState,
                _: &mut Self::PrepaintState,
                window: &mut mozui::Window,
                cx: &mut mozui::App,
            ) {
                <Self as Plot>::paint(self, bounds, window, cx)
            }
        }
    };

    TokenStream::from(expanded)
}
