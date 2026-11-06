use super::*;
use windows::System::{DispatcherQueue, DispatcherQueueController};
use windows::UI::Composition::{CompositionBrush, Compositor, ICompositionSupportsSystemBackdrop};
use windows::Win32::System::WinRT::{
    CreateDispatcherQueueController, DQTAT_COM_STA, DQTYPE_THREAD_CURRENT, DispatcherQueueOptions,
};
use windows::core::Interface as WindowsInterface;

thread_local! {
    static SYSTEM_COMPOSITOR: RefCell<Weak<SystemCompositor>> = const { RefCell::new(Weak::new()) };
}

struct SystemCompositor {
    compositor: Compositor,
    _dispatcher: Option<DispatcherQueueController>,
}

impl SystemCompositor {
    fn get() -> windows::core::Result<Rc<Self>> {
        SYSTEM_COMPOSITOR.with(|shared| {
            if let Some(compositor) = shared.borrow().upgrade() {
                return Ok(compositor);
            }
            let dispatcher = if DispatcherQueue::GetForCurrentThread().is_err() {
                Some(unsafe {
                    CreateDispatcherQueueController(DispatcherQueueOptions {
                        dwSize: size_of::<DispatcherQueueOptions>() as u32,
                        threadType: DQTYPE_THREAD_CURRENT,
                        apartmentType: DQTAT_COM_STA,
                    })
                }?)
            } else {
                None
            };
            let compositor = Rc::new(Self {
                compositor: Compositor::new()?,
                _dispatcher: dispatcher,
            });
            *shared.borrow_mut() = Rc::downgrade(&compositor);
            Ok(compositor)
        })
    }
}

/// Shares the system compositor across windows and clears the brush before releasing it.
pub(super) struct TransparentBackdrop {
    target: Option<ICompositionSupportsSystemBackdrop>,
    _brush: CompositionBrush,
    _compositor: Rc<SystemCompositor>,
}

impl TransparentBackdrop {
    pub(super) fn new(window: &native::Window) -> Result<Self, WinUiError> {
        let create = || -> windows::core::Result<Self> {
            // The two windows-rs versions wrap the same COM ABI. Borrow the native interface;
            // QueryInterface creates the owned reference retained by this guard.
            let raw = Interface::as_raw(window);
            let window = unsafe { windows::core::IUnknown::from_raw_borrowed(&raw) }.unwrap();
            let target = window.cast::<ICompositionSupportsSystemBackdrop>()?;
            let compositor = SystemCompositor::get()?;
            let brush: CompositionBrush = compositor
                .compositor
                .CreateColorBrushWithColor(windows::UI::Color {
                    A: 0,
                    R: 0,
                    G: 0,
                    B: 0,
                })?
                .cast()?;
            target.SetSystemBackdrop(&brush)?;
            Ok(Self {
                target: Some(target),
                _brush: brush,
                _compositor: compositor,
            })
        };
        create().map_err(|error| {
            WinUiError::Native(windows_core::Error::from_hresult(HRESULT(error.code().0)))
        })
    }

    /// A closed WinUI window has already released its native backing state. Calling its backdrop
    /// setter at that point can access freed state, even while the COM interface remains alive.
    pub(super) fn window_closed(&mut self) {
        self.target.take();
    }

    pub(super) fn clear(&mut self) {
        // Detach while the window is live, but retain the brush and compositor through teardown.
        if let Some(target) = self.target.take() {
            let _ = target.SetSystemBackdrop(None::<&CompositionBrush>);
        }
    }
}

impl Drop for TransparentBackdrop {
    fn drop(&mut self) {
        self.clear();
    }
}
