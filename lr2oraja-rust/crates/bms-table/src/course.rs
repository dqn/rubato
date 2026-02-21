use crate::bms_table_element::BmsTableElement;

#[derive(Debug, Clone)]
pub struct Course {
    name: String,
    charts: Vec<BmsTableElement>,
    style: String,
    constraint: Vec<String>,
    trophy: Vec<Trophy>,
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

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn get_charts(&self) -> &[BmsTableElement] {
        &self.charts
    }

    pub fn set_charts(&mut self, charts: Vec<BmsTableElement>) {
        self.charts = charts;
    }

    pub fn get_style(&self) -> &str {
        &self.style
    }

    pub fn set_style(&mut self, style: &str) {
        self.style = style.to_string();
    }

    pub fn get_constraint(&self) -> &[String] {
        &self.constraint
    }

    pub fn set_constraint(&mut self, constraint: Vec<String>) {
        self.constraint = constraint;
    }

    pub fn get_trophy(&self) -> &[Trophy] {
        &self.trophy
    }

    pub fn set_trophy(&mut self, trophy: Vec<Trophy>) {
        self.trophy = trophy;
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
    scorerate: f64,
    missrate: f64,
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

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn get_style(&self) -> &str {
        &self.style
    }

    pub fn set_style(&mut self, style: &str) {
        self.style = style.to_string();
    }

    pub fn get_scorerate(&self) -> f64 {
        self.scorerate
    }

    pub fn set_scorerate(&mut self, scorerate: f64) {
        self.scorerate = scorerate;
    }

    pub fn get_missrate(&self) -> f64 {
        self.missrate
    }

    pub fn set_missrate(&mut self, missrate: f64) {
        self.missrate = missrate;
    }
}

impl Default for Trophy {
    fn default() -> Self {
        Self::new()
    }
}
