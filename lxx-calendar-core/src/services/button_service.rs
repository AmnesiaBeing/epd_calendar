use lxx_calendar_common::*;

pub struct ButtonService<D: ButtonDriver> {
    initialized: bool,
    button_device: Option<D>,
    event_sender: Option<LxxChannelSender<'static, SystemEvent>>,
}

impl<D: ButtonDriver> ButtonService<D> {
    pub fn new(sender: LxxChannelSender<'static, SystemEvent>) -> Self {
        Self {
            initialized: false,
            button_device: None,
            event_sender: Some(sender),
        }
    }

    pub fn set_button_device(&mut self, device: D) {
        self.button_device = Some(device);
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing button service");

        if let Some(ref mut device) = self.button_device {
            let sender = self.event_sender.clone();
            device
                .register_press_callback(move |event| {
                    if let Some(ref s) = sender {
                        let user_event = match event {
                            ButtonEvent::DoubleClick => UserEvent::ButtonDoubleClick,
                            ButtonEvent::TripleClick => UserEvent::ButtonTripleClick,
                            ButtonEvent::ShortPress => UserEvent::ButtonShortPress,
                            ButtonEvent::LongPress => UserEvent::ButtonLongPress,
                        };
                        let _ = s.try_send(SystemEvent::UserEvent(user_event));
                    }
                })
                .await
                .ok();
        }

        self.initialized = true;
        info!("Button service initialized");
        Ok(())
    }

    // TODO: 可能需要一个 release 方法来释放资源，或者在 Drop 中实现
    pub async fn _release(&mut self) -> SystemResult<()> {
        info!("Release button service");
        Ok(())
    }
}
