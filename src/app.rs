#[derive(Debug, Default, Clone)]
pub struct App {
    page: Page,
}

#[derive(Debug, Default, Clone)]
pub enum Page {
    Mods,
    #[default]
    Settings,
    Categories,
}

pub enum Message {}

impl App {
    fn new() -> Self {
        todo!()
    }

    fn update(&mut self, message: Message) {
        match message {}
    }
}
