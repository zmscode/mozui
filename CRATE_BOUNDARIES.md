# Crate Boundaries

## Intended Split

### `mozui`

`mozui` is the framework core.

It owns:

- app, window, entity, and runtime lifecycle
- rendering, layout, scene, and input systems
- primitive elements such as `div`, text, image, canvas, list, and surface
- geometry and positioning primitives
- platform abstractions and backend implementations
- native control substrate
- native window chrome and hosted-surface substrate

### `mozui-components`

`mozui-components` is the semantic UI layer built on top of `mozui`.

It owns:

- semantic controls and composed UI patterns
- design-system styling helpers and theme-driven presentation
- dialogs, sheets, popovers, menus, sidebars, tables, forms, notifications, and other application-facing components
- convenience APIs like `.native()` that choose between semantic and native-backed rendering

## Practical Rule

If a concept is:

- about runtime, layout, geometry, positioning, platform behavior, or native substrate, it belongs in `mozui`
- about product-facing widgets, composed UI, or design-system semantics, it belongs in `mozui-components`

## Current Boundary Cleanup

The first concrete cleanup in this direction is now complete:

- `Anchor` moved into `mozui`
- the richer anchored positioning behavior moved into `mozui`
- `mozui-components` no longer owns a fork of the anchored primitive and instead reuses the core implementation

This is the model to follow for future cleanup:

- move primitives downward into `mozui`
- keep semantic composition upward in `mozui-components`
- avoid reintroducing platform or positioning substrate into `mozui-components`
