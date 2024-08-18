use crate::aws::InstanceInfo;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::{Render, View};

#[derive(Debug, Clone, Default)]
pub struct InstanceDetails {
    instance: Option<InstanceInfo>,
}

impl InstanceDetails {
    pub fn set_instance(&mut self, instance: InstanceInfo) {
        self.instance = Some(instance);
    }
}

#[allow(refining_impl_trait)]
impl View for InstanceDetails {
    fn get_widget(&self) -> Paragraph {
        let text = match &self.instance {
            Some(instance) => {
                let data = vec![
                    ("Name", instance.get_name()),
                    ("Instance Id", instance.get_instance_id()),
                    ("Private IP", instance.get_private_ip()),
                    ("Public IP", instance.get_public_ip()),
                    ("Image Id", instance.get_image_id()),
                    ("instance_type", instance.get_instance_type()),
                    ("launch_time", instance.get_launch_time()),
                    ("vpc_id", instance.get_vpc_id()),
                    (
                        "security_groups",
                        format!("{:#?}", instance.get_security_groups()),
                    ),
                    ("tags", format!("{:#?}", instance.get_tags())),
                ];
                let text = data
                    .iter()
                    .map(|(key, value)| format!("{}: {}", key, value))
                    .collect::<Vec<String>>()
                    .join("\n");
                Text::from(text)
            }
            None => Text::from("No instance selected".to_string()),
        };
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Details"))
    }
}

impl Render for InstanceDetails {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let widget = self.get_widget();
        frame.render_widget(widget, area);
    }
}
