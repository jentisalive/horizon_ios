//! Touch uses the existing gameplay mappings and menu pointer handlers.
use super::{Game, Screen, event_loop::EventLoopCtl};
use horizon_client::hud::{HudBatch, rgba};
use horizon_client_input::touch::{self, Action, Button, Phase};
use winit::{
    event::{DeviceId, ElementState, MouseButton, Touch, TouchPhase, WindowEvent},
    window::WindowId,
};

impl Game<'_> {
    pub(super) fn touch_playing(&self) -> bool {
        self.screen == Screen::Playing
            && self.title.is_none()
            && self.player.is_some()
            && !self.dialog_open()
            && self.server_packs.prompt.is_none()
            && self.pending_save.is_none()
            && self.world_loader.is_none()
    }
    pub(super) fn touch_action(&mut self, action: Action) {
        use winit::keyboard::KeyCode;
        let result = match action {
            Action::Look([x, y]) => {
                self.turn_player(f64::from(x) * 2.0, f64::from(y) * 2.0);
                Ok(())
            }
            Action::Slot(slot) => self
                .mapped_key(super::HOTBAR_DIGITS[slot], true, false)
                .and_then(|_| self.mapped_key(super::HOTBAR_DIGITS[slot], false, false)),
            Action::Tap(Button::Pause) => self.pad_back(),
            Action::Tap(Button::Inventory) => self
                .mapped_key(KeyCode::KeyE, true, false)
                .and_then(|_| self.mapped_key(KeyCode::KeyE, false, false)),
            Action::Hold(Button::Attack, down) => self.play_mouse(MouseButton::Left, down),
            Action::Hold(Button::Use, down) => self.play_mouse(MouseButton::Right, down),
            Action::Hold(button, down) => {
                let key = match button {
                    Button::Forward => KeyCode::KeyW,
                    Button::Back => KeyCode::KeyS,
                    Button::Left => KeyCode::KeyA,
                    Button::Right => KeyCode::KeyD,
                    Button::Jump => KeyCode::Space,
                    Button::Sneak => KeyCode::ShiftLeft,
                    _ => return,
                };
                self.mapped_key(key, down, false)
            }
            _ => Ok(()),
        };
        if let Err(e) = result {
            log::warn!("Horizon touch: {e}");
        }
    }
    pub(super) fn reset_touch(&mut self) {
        for action in self.touch.reset() {
            self.touch_action(action);
        }
        self.touch_menu_finger = None;
    }
    pub(super) fn touch_event(&mut self, event_loop: &dyn EventLoopCtl, id: WindowId, t: Touch) {
        self.last_input = std::time::Instant::now();
        let Some(r) = &self.renderer else {
            return;
        };
        let b = HudBatch::new([r.config.width, r.config.height]);
        let p = [t.location.x as f32 / b.scale, t.location.y as f32 / b.scale];
        if self.touch_playing() {
            let phase = match t.phase {
                TouchPhase::Started => Phase::Start,
                TouchPhase::Moved => Phase::Move,
                TouchPhase::Ended | TouchPhase::Cancelled => Phase::End,
            };
            for action in self.touch.event(t.id, phase, p, [b.width, b.height]) {
                self.touch_action(action);
            }
            if !self.touch_playing() {
                self.reset_touch();
            }
        } else {
            // One finger drives menu clicks and drags. Other fingers cannot release it.
            if t.phase == TouchPhase::Started && self.touch_menu_finger.is_none() {
                self.touch_menu_finger = Some(t.id);
            }
            if self.touch_menu_finger != Some(t.id) {
                return;
            }
            self.handle_window_event(
                event_loop,
                id,
                WindowEvent::CursorMoved {
                    device_id: DeviceId::dummy(),
                    position: t.location,
                },
            );
            let state = match t.phase {
                TouchPhase::Started => Some(ElementState::Pressed),
                TouchPhase::Ended | TouchPhase::Cancelled => Some(ElementState::Released),
                _ => None,
            };
            if let Some(state) = state {
                self.handle_window_event(
                    event_loop,
                    id,
                    WindowEvent::MouseInput {
                        device_id: DeviceId::dummy(),
                        state,
                        button: MouseButton::Left,
                    },
                );
            }
            if self.touch_playing() && self.touch_menu_finger.take().is_some() {
                self.handle_window_event(
                    event_loop,
                    id,
                    WindowEvent::MouseInput {
                        device_id: DeviceId::dummy(),
                        state: ElementState::Released,
                        button: MouseButton::Left,
                    },
                );
            }
            if matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.touch_menu_finger = None;
            }
        }
    }
    pub(super) fn draw_touch_controls(&mut self, batch: &mut HudBatch) {
        if !self.touch_playing()
            || !(cfg!(target_os = "ios") || std::env::var_os("HORIZON_TOUCH").is_some())
        {
            return;
        }
        for (button, touch::Rect(x, y, w, h)) in touch::buttons([batch.width, batch.height]) {
            let color = if self.touch.held(button) {
                rgba(220, 220, 220, 170)
            } else {
                rgba(35, 35, 35, 125)
            };
            batch.fill(&self.gui.atlas, x, y, w, h, color);
            let label = match button {
                Button::Forward => "^",
                Button::Back => "v",
                Button::Left => "<",
                Button::Right => ">",
                Button::Jump => "+",
                Button::Sneak => "_",
                Button::Attack => "X",
                Button::Use => "O",
                Button::Inventory => "...",
                Button::Pause => "||",
                Button::Scheme => {
                    if self.touch.dpad {
                        "#"
                    } else {
                        "+"
                    }
                }
            };
            batch.text(&self.gui.atlas, label, x + 8.0, y + 8.0, u32::MAX, true);
        }
    }
}
