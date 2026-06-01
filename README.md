Very fast, Rust-based terminal rendering for JavaScript and TypeScript.
PaintCannon exposes a DOM API subset via NAPI-RS bindings to JS and renders to
terminal backends very quickly.

We also ship `paintcannon-react`, a React reconciler that sits on top of
PaintCannon's DOM API, for fast, React-based terminal rendering.

To check out the main demo, clone this repo and then:

```bash
npm run build:debug
npm run demo:todo
```

This builds the main PaintCannon API bindings, and runs the React-based TODO
app in your terminal with scrollbars, clickable buttons, keyboard shortcuts,
and more.
