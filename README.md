# egui_hooks

React Hooks like API for egui

## Status

- [x] precise memory management (double buffered)
- [x] use_state
- [x] use_persistent_state
- [x] use_memo
- [ ] use_cache (a thin wrapper of caches in `egui::Memory`)
- [x] use_effect
- [ ] use_future (needs async runtime)

## Usage

1. use_state

```rust
// You can reset the initial state by changing the dependency part.
let count = ui.use_state(0usize, ());
ui.label(format!("Count: {}", count));
if ui.button("Increment").clicked() {
    count.set_next(*count + 1);
}
```

2. use_memo

```rust
let count = ui.use_state(0usize, ());
let memo = ui.use_memo(
    || {
        println!("Calculating memoized value");
        count.pow(2)
    },
    count.clone(),
);
ui.label(format!("Memo: {}", memo));
if ui.button("Increment").clicked() {
    count.set_next(*count + 1);
}
```
