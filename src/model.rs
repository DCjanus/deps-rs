use badge::BadgeOptions;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Site {
    GitHub,
    GitLab,
    BitBucket,
}

#[derive(Debug, Deserialize)]
pub struct Identity {
    pub site: Site,
    pub owner: String,
    pub repo: String,
}

#[derive(Debug)]
pub enum Status {
    Unknown,
    Known { total: u32, outdated: u32 },
}

impl Status {
    pub fn to_svg(&self) -> String {
        let badge_options = match self {
            Status::Unknown => BadgeOptions {
                subject: "dependencies".into(),
                status: "unknown".into(),
                color: "#9f9f9f".into(),
            },
            Status::Known { total, outdated } => {
                if *outdated > 0 {
                    BadgeOptions {
                        subject: "dependencies".into(),
                        status: format!("{} of {} outdated", outdated, total),
                        color: "#dfb317".into(),
                    }
                } else if *total > 0 {
                    BadgeOptions {
                        subject: "dependencies".into(),
                        status: "up to date".into(),
                        color: "#4c1".into(),
                    }
                } else {
                    BadgeOptions {
                        subject: "dependencies".into(),
                        status: "none".into(),
                        color: "#4c1".into(),
                    }
                }
            }
        };

        badge::Badge::new(badge_options).unwrap().to_svg()
    }
}
