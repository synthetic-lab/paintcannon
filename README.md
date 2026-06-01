Very fast, Rust-based terminal rendering for JavaScript and TypeScript.
PaintCannon exposes a DOM API subset via NAPI-RS bindings to JS and renders to
terminal backends very quickly.

PaintCannon supports the following CSS:

* FlexBox
* Grid
* Block
* Inline
* Margins and padding, including auto margins
* Overflow hidden and scroll, with native mouse scrolling
* RGB background, border, text, placeholder, and selection coloring with
  limited-palette fallbacks
* CSS transitions for all color properties
* Mouse pointer styling

It also exposes a non-standard set of border styles, since typical
pixel-based borders can't be rendered in terminals:

* `none`
* `solid`
* `double`
* `heavy`
* `rounded`
* `chunky-rounded`
* `ascii`

PaintCannon supports the following HTML elements:

* `<div>`
* `<span>`
* `<input type="text">`
* `<button>`
* `<form>`
* `<img>` (via ANSI/ASCII rendering)

And the following DOM events and event handlers, with bubbling,
`stopPropagation`, and `preventDefault` support:

* Click events
* Mouse enter and leave events
* Keyboard events
* Input change events
* Form submit events
* Focus and blur events
* Transition start and end events
* Scroll events

PaintCannon also exposes a `requestAnimationFrame` function to hook into its
paint timing, much like browsers do.

We also ship `paintcannon-react`, a React reconciler that sits on top of
PaintCannon's DOM API, for fast, React-based terminal rendering with
[Ink](https://github.com/vadimdemedes/ink)-inspired hooks.

To check out the main demo, clone this repo and then:

```bash
npm run build:debug
npm run demo:todo
```

This builds the main PaintCannon API bindings, and runs the React-based TODO
app in your terminal with scrollbars, clickable buttons, keyboard shortcuts,
and more. If you want even more speed:

```bash
npm run build # release build
node ./paintcannon-react/dist/examples/todo.js
```
