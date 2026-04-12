# Architecture

## What you should know

- [Bevy ECS](https://bevy.org/learn/quick-start/getting-started/ecs/): entity–component–system runtime that drives the native shell, scheduling, and how CEF webviews sit in the scene.
- [Dioxus](https://dioxuslabs.com/learn/0.7/): declarative Rust UI used for embedded surfaces (e.g. history) shipped as HTML/WASM inside the webview stack.

## Entity Design

```mermaid
flowchart TB
  subgraph layout[Layout Components]
    Space --> Window
    Window --> Pane
    Pane --> Tab
  end
  subgraph tab[Tab Components]
    Tab --> Home
    Home --> CEF
    Tab --> Browser
    Browser --> CEF
    Tab --> History
    Tab --> Settings
    Tab --> Help["Help (cheatsheet)"]
    Browser --> Web["CEF webview (page content)"]
  end
```

## Webview App

`vmux_webview_app` serves a small front-end bundle over a custom `vmux://` scheme: assets under `dist/` are embedded into Bevy, CEF loads `index.html` for a named host. 

```mermaid
flowchart LR
  subgraph build["Front-end build"]
    Dist["dist/ (HTML, wasm, assets)"]
  end
  subgraph plugin["WebviewAppPlugin"]
    Embed["Embed into Bevy EmbeddedAssetRegistry"]
    Config["CefEmbeddedPageConfig: scheme + host + default document"]
  end
  subgraph runtime["Runtime"]
    CEF["CEF webview"]
    Page["Loaded page"]
    Emit["JsEmitEvent / host bridge"]
    Bevy["Bevy systems & observers"]
  end
  Dist --> Embed
  Embed --> Config
  Config --> CEF
  CEF --> Page
  Page --> Emit
  Emit --> Bevy
```

