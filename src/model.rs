#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Site {
    GitHub,
    GitLab,
}

#[derive(Debug, Deserialize)]
pub struct Identity {
    repo: String,
    owner: String,
    site: Site,
}

#[derive(Debug)]
pub struct Status {
    pub total: u32,
    pub outdated: u32,
}

impl Status {
    pub fn to_svg(&self) -> String {
        let (color, status) = if self.outdated > 0 {
            (
                "#dfb317".to_string(),
                format!("{} of {} outdated", self.outdated, self.total),
            )
        } else {
            ("#4c1".to_string(), "up to date".to_string())
        };

        badge::Badge::new(badge::BadgeOptions {
            subject: "dependencies".to_string(),
            status,
            color,
        })
        .unwrap()
        .to_svg()
    }
}
