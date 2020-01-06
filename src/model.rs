use std::{iter::Sum, ops::Add};

use actix_web::{HttpRequest, HttpResponse, Responder};
use badge::BadgeOptions;
use futures::future::Ready;
use semver::Version;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Site {
    GitHub,
    GitLab,
    BitBucket,
}

#[derive(Debug, Deserialize)]
pub struct RepoIdentity {
    pub site: Site,
    pub owner: String,
    pub repo: String,
}

#[derive(Debug, Deserialize)]
pub struct CrateIdentity {
    pub name: String,
    pub version: Version,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Status {
    Unknown,
    Insecure,
    Normal { total: u32, outdated: u32 },
}

impl Sum for Status {
    fn sum<I: Iterator<Item = Self>>(mut iter: I) -> Self {
        let mut result = match iter.next() {
            None => Status::Unknown,
            Some(x) => x,
        };

        for i in iter {
            result = result + i;
        }

        result
    }
}

impl Add for Status {
    type Output = Status;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (_, Status::Insecure) | (Status::Insecure, _) => Status::Insecure,
            (Status::Unknown, _) | (_, Status::Unknown) => Status::Unknown,
            (
                Status::Normal {
                    total: total1,
                    outdated: outdated1,
                },
                Status::Normal {
                    total: total2,
                    outdated: outdated2,
                },
            ) => Status::Normal {
                total: total1 + total2,
                outdated: outdated1 + outdated2,
            },
        }
    }
}

impl Responder for Status {
    type Error = ();
    type Future = Ready<Result<HttpResponse, ()>>;

    fn respond_to(self, _: &HttpRequest) -> Self::Future {
        let response = HttpResponse::Ok()
            .content_type("image/svg+xml;charset=utf-8")
            .body(self.to_svg());

        futures::future::ready(Ok(response))
    }
}

impl Status {
    pub fn to_svg(&self) -> String {
        let badge_options = match self {
            Status::Unknown => BadgeOptions {
                subject: "dependencies".into(),
                status: "unknown".into(),
                color: "#9f9f9f".into(),
            },
            Status::Normal { total, outdated } => {
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
            Status::Insecure => BadgeOptions {
                subject: "dependencies".into(),
                status: "insecure".into(),
                color: "#e05d44".into(),
            },
        };

        badge::Badge::new(badge_options).unwrap().to_svg()
    }
}
