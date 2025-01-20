use std::time::Duration;

use gpui::{ModelContext, Timer};

static INTERVAL: Duration = Duration::from_millis(500);
static PAUSE_DELAY: Duration = Duration::from_millis(300);

pub(crate) struct BlinkCursor {
    visible: bool,
    paused: bool,
    epoch: usize,
}

impl BlinkCursor {
    pub fn new() -> Self {
        Self {
            visible: false,
            paused: false,
            epoch: 0,
        }
    }

    pub fn start(&mut self, cx: &mut ModelContext<Self>) {
        self.blink(self.epoch, cx);
    }

    pub fn stop(&mut self, cx: &mut ModelContext<Self>) {
        self.visible = false;
        self.epoch = 0;
        cx.notify();
    }

    fn next_epoch(&mut self) -> usize {
        self.epoch += 1;
        self.epoch
    }

    fn blink(&mut self, epoch: usize, cx: &mut ModelContext<Self>) {
        if self.paused || epoch != self.epoch {
            return;
        }

        self.visible = !self.visible;
        cx.notify();

        let epoch = self.next_epoch();
        cx.spawn(|this, mut cx| async move {
            Timer::after(INTERVAL).await;
            if let Some(this) = this.upgrade() {
                this.update(&mut cx, |this, cx| this.blink(epoch, cx)).ok();
            }
        })
        .detach();
    }

    pub fn visible(&self) -> bool {
        // Keep showing the cursor if paused
        self.paused || self.visible
    }

    pub fn pause(&mut self, cx: &mut ModelContext<Self>) {
        self.paused = true;
        cx.notify();

        // delay 500ms to start the blinking
        let epoch = self.next_epoch();
        cx.spawn(|this, mut cx| async move {
            Timer::after(PAUSE_DELAY).await;

            if let Some(this) = this.upgrade() {
                this.update(&mut cx, |this, cx| {
                    this.paused = false;
                    this.blink(epoch, cx);
                })
                .ok();
            }
        })
        .detach();
    }
}
