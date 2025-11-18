# electron-macos-window-extensions

macOS window management extensions for Electron applications.

## Installation

```bash
npm install electron-macos-window-extensions
```

## API

### WindowObserver

Observe window key status changes.

```js
import { WindowObserver } from 'electron-macos-window-extensions'

const observer = new WindowObserver(
  window.getNativeWindowHandle(),
  (event) => {
    console.log(event.type) // 'BecomeKey' | 'ResignKey'
  }
)

observer.stop()
```

### Window Controls

```js
import { showAndMakeKey, orderOut, nonActivatingPanel, setAnimationBehavior, deactivateApp } from 'electron-macos-window-extensions'

// Display window and make it the key window (receives keyboard events).
showAndMakeKey(window.getNativeWindowHandle())

// Hide window (resigns key)
orderOut(window.getNativeWindowHandle())

// Configure as non-activating panel (interactions won't activate the app)
nonActivatingPanel(window.getNativeWindowHandle(), { resizable: true })

// Set window animation behavior
// Options: Default, None, DocumentWindow, UtilityWindow, AlertPanel
setAnimationBehavior(window.getNativeWindowHandle(), 'None')

// Deactivate application (move focus to another app)
deactivateApp()
```

## License
MIT
