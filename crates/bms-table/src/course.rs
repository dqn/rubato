use crate::bms_table_element::BmsTableElement;

#[derive(Debug, Clone)]
pub struct Course {
    name: String,
    pub charts: Vec<BmsTableElement>,
    style: String,
    pub constraint: Vec<String>,
    pub trophy: Vec<Trophy>,
}

impl Course {
    pub fn new() -> Self {
        Self {
            name: "\u{65b0}\u{898f}\u{6bb5}\u{4f4d}".to_string(),
            charts: Vec::new(),
            style: String::new(),
            constraint: Vec::new(),
            trophy: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn charts(&self) -> &[BmsTableElement] {
        &self.charts
    }
    pub fn get_style(&self) -> &str {
        &self.style
    }

    pub fn set_style(&mut self, style: &str) {
        self.style = style.to_string();
    }

    pub fn constraint(&self) -> &[String] {
        &self.constraint
    }
    pub fn get_trophy(&self) -> &[Trophy] {
        &self.trophy
    }
}

impl Default for Course {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Trophy {
    name: String,
    style: String,
    pub scorerate: f64,
    pub missrate: f64,
}

impl Trophy {
    pub fn new() -> Self {
        Self {
            name: "\u{65b0}\u{898f}\u{30c8}\u{30ed}\u{30d5}\u{30a3}\u{30fc}".to_string(),
            style: String::new(),
            scorerate: 0.0,
            missrate: 100.0,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn style(&self) -> &str {
        &self.style
    }

    pub fn set_style(&mut self, style: &str) {
        self.style = style.to_string();
    }

    pub fn scorerate(&self) -> f64 {
        self.scorerate
    }
    pub fn get_missrate(&self) -> f64 {
        self.missrate
    }
}

impl Default for Trophy {
    fn default() -> Self {
        Self::new()
    }
}
