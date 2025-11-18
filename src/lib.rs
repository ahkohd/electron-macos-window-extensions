#![deny(clippy::all)]

use block2::RcBlock;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSView, NSWindowAnimationBehavior, NSWindowStyleMask};
use objc2_foundation::{MainThreadMarker, NSNotification, NSNotificationCenter};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::Arc;

/// Window key window status changes.
#[napi(string_enum)]
#[derive(Clone, Copy)]
pub enum WindowEventType {
  /// Window resigned key window status (no longer receives keyboard events).
  ResignKey,
  /// Window became the key window (now receives keyboard events).
  BecomeKey,
}

/// Emitted when window key window status changes.
#[napi]
pub struct WindowEvent {
  pub r#type: WindowEventType,
}

struct WindowObserverState {
  observers: [AtomicPtr<objc2::runtime::AnyObject>; 2],
  should_stop: Arc<AtomicBool>,
}

unsafe impl Send for WindowObserverState {}
unsafe impl Sync for WindowObserverState {}

#[napi]
pub struct WindowObserver {
  state: Arc<WindowObserverState>,
}

#[napi]
impl WindowObserver {
  #[napi(constructor)]
  pub fn new(
    window_handle: napi::bindgen_prelude::Buffer,
    callback: ThreadsafeFunction<WindowEvent>,
  ) -> napi::Result<Self> {
    let should_stop = Arc::new(AtomicBool::new(false));
    let callback = Arc::new(callback);

    let state = Arc::new(WindowObserverState {
      observers: [
        AtomicPtr::new(std::ptr::null_mut()),
        AtomicPtr::new(std::ptr::null_mut()),
      ],
      should_stop: should_stop.clone(),
    });

    // Read NSView pointer value from buffer data (Electron returns view, not window)
    let view_ptr = unsafe { std::ptr::read(window_handle.as_ptr() as *const usize) };

    // Register observers on current thread (main thread from Electron)
    unsafe {
      let view = &*(view_ptr as *mut NSView);

      // Get the NSWindow from the NSView
      let window = match view.window() {
        Some(w) => w,
        None => return Err(napi::Error::from_reason("View has no window")),
      };

      let center = NSNotificationCenter::defaultCenter();

      // Observe ResignKey
      {
        let should_stop_clone = should_stop.clone();
        let callback_clone = callback.clone();
        let block = RcBlock::new(move |_notification: NonNull<NSNotification>| {
          if should_stop_clone.load(Ordering::Acquire) {
            return;
          }
          let event = WindowEvent {
            r#type: WindowEventType::ResignKey,
          };
          let _ = callback_clone.call(Ok(event), ThreadsafeFunctionCallMode::NonBlocking);
        });

        let observer = center.addObserverForName_object_queue_usingBlock(
          Some(&objc2_foundation::NSString::from_str(
            "NSWindowDidResignKeyNotification",
          )),
          Some(&window),
          None,
          &block,
        );

        let observer_ptr = Retained::as_ptr(&observer) as *mut objc2::runtime::AnyObject;
        state.observers[0].store(observer_ptr, Ordering::Release);
        std::mem::forget(observer);
      }

      // Observe BecomeKey
      {
        let should_stop_clone = should_stop.clone();
        let callback_clone = callback.clone();
        let block = RcBlock::new(move |_notification: NonNull<NSNotification>| {
          if should_stop_clone.load(Ordering::Acquire) {
            return;
          }
          let event = WindowEvent {
            r#type: WindowEventType::BecomeKey,
          };
          let _ = callback_clone.call(Ok(event), ThreadsafeFunctionCallMode::NonBlocking);
        });

        let observer = center.addObserverForName_object_queue_usingBlock(
          Some(&objc2_foundation::NSString::from_str(
            "NSWindowDidBecomeKeyNotification",
          )),
          Some(&window),
          None,
          &block,
        );

        let observer_ptr = Retained::as_ptr(&observer) as *mut objc2::runtime::AnyObject;
        state.observers[1].store(observer_ptr, Ordering::Release);
        std::mem::forget(observer);
      }
    }

    Ok(WindowObserver { state })
  }

  #[napi]
  pub fn stop(&mut self) -> napi::Result<()> {
    self.state.should_stop.store(true, Ordering::Release);

    for observer_atomic in &self.state.observers {
      let observer_ptr = observer_atomic.swap(std::ptr::null_mut(), Ordering::AcqRel);
      if !observer_ptr.is_null() {
        unsafe {
          let observer = Retained::<objc2::runtime::AnyObject>::from_raw(observer_ptr).unwrap();
          NSNotificationCenter::defaultCenter().removeObserver(&observer);
        }
      }
    }

    Ok(())
  }
}

/// Extract NSView reference from Electron window handle buffer.
fn get_view(window_handle: &napi::bindgen_prelude::Buffer) -> napi::Result<&NSView> {
  let view_ptr = unsafe { std::ptr::read(window_handle.as_ptr() as *const usize) };
  Ok(unsafe { &*(view_ptr as *mut NSView) })
}

/// Get NSWindow from NSView, returning error if view has no window.
fn get_window(view: &NSView) -> napi::Result<Retained<objc2_app_kit::NSWindow>> {
  view
    .window()
    .ok_or_else(|| napi::Error::from_reason("View has no window"))
}

/// Display window and make it the key window (receives keyboard events).
#[napi]
pub fn show_and_make_key(window_handle: napi::bindgen_prelude::Buffer) -> napi::Result<()> {
  let view = get_view(&window_handle)?;
  let window = get_window(view)?;

  window.makeFirstResponder(Some(view));
  window.orderFrontRegardless();
  window.makeKeyWindow();

  Ok(())
}

/// Remove window from screen list.
#[napi]
pub fn order_out(window_handle: napi::bindgen_prelude::Buffer) -> napi::Result<()> {
  let view = get_view(&window_handle)?;
  let window = get_window(view)?;

  window.orderOut(None);

  Ok(())
}

/// Configuration for non-activating panel.
#[napi(object)]
pub struct NonActivatingPanelConfig {
  pub resizable: bool,
}

/// Set window as non-activating panel (interactions won't activate the app).
#[napi]
pub fn non_activating_panel(
  window_handle: napi::bindgen_prelude::Buffer,
  config: Option<NonActivatingPanelConfig>,
) -> napi::Result<()> {
  let view = get_view(&window_handle)?;
  let window = get_window(view)?;

  let mut mask = NSWindowStyleMask::NonactivatingPanel;
  if let Some(cfg) = config {
    if cfg.resizable {
      mask |= NSWindowStyleMask::Resizable;
    }
  }

  window.setStyleMask(mask);

  Ok(())
}

/// Deactivate application (move focus to another app).
#[napi]
pub fn deactivate_app() -> napi::Result<()> {
  let mtm = MainThreadMarker::new()
    .ok_or_else(|| napi::Error::from_reason("Must be called from main thread"))?;

  let app = NSApplication::sharedApplication(mtm);

  app.deactivate();

  Ok(())
}

#[napi(string_enum)]
#[derive(Clone, Copy)]
pub enum AnimationBehavior {
  Default,
  None,
  DocumentWindow,
  UtilityWindow,
  AlertPanel,
}

impl From<AnimationBehavior> for NSWindowAnimationBehavior {
  fn from(behavior: AnimationBehavior) -> Self {
    match behavior {
      AnimationBehavior::Default => NSWindowAnimationBehavior::Default,
      AnimationBehavior::None => NSWindowAnimationBehavior::None,
      AnimationBehavior::DocumentWindow => NSWindowAnimationBehavior::DocumentWindow,
      AnimationBehavior::UtilityWindow => NSWindowAnimationBehavior::UtilityWindow,
      AnimationBehavior::AlertPanel => NSWindowAnimationBehavior::AlertPanel,
    }
  }
}

/// Set window animation behavior for ordering on/off screen.
#[napi]
pub fn set_animation_behavior(
  window_handle: napi::bindgen_prelude::Buffer,
  behavior: AnimationBehavior,
) -> napi::Result<()> {
  let view = get_view(&window_handle)?;
  let window = get_window(view)?;

  window.setAnimationBehavior(behavior.into());

  Ok(())
}
