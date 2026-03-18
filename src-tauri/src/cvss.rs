use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AttackVector {
    Network,
    Adjacent,
    Local,
    Physical,
}
impl AttackVector {
    fn value(self) -> f32 {
        match self {
            Self::Network => 0.85,
            Self::Adjacent => 0.62,
            Self::Local => 0.55,
            Self::Physical => 0.2,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AttackComplexity {
    Low,
    High,
}
impl AttackComplexity {
    fn value(self) -> f32 {
        match self {
            Self::Low => 0.77,
            Self::High => 0.44,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PrivilegesRequired {
    None,
    Low,
    High,
}
impl PrivilegesRequired {
    fn value(self, scope: Scope) -> f32 {
        match (self, scope) {
            (Self::None, _) => 0.85,
            (Self::Low, Scope::Unchanged) => 0.62,
            (Self::Low, Scope::Changed) => 0.68,
            (Self::High, Scope::Unchanged) => 0.27,
            (Self::High, Scope::Changed) => 0.5,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UserInteraction {
    None,
    Required,
}
impl UserInteraction {
    fn value(self) -> f32 {
        match self {
            Self::None => 0.85,
            Self::Required => 0.62,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Scope {
    Unchanged,
    Changed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Impact {
    None,
    Low,
    High,
}
impl Impact {
    fn value(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Low => 0.22,
            Self::High => 0.56,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CvssV3 {
    pub av: AttackVector,
    pub ac: AttackComplexity,
    pub pr: PrivilegesRequired,
    pub ui: UserInteraction,
    pub s: Scope,
    pub c: Impact,
    pub i: Impact,
    pub a: Impact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CvssScore {
    pub base_score: f32,
    pub severity: String,
}

impl CvssV3 {
    pub fn base_score(&self) -> f32 {
        let iss = 1.0 - (1.0 - self.c.value()) * (1.0 - self.i.value()) * (1.0 - self.a.value());
        let impact = match self.s {
            Scope::Unchanged => 6.42 * iss,
            Scope::Changed => 7.52 * (iss - 0.029) - 3.25 * (iss - 0.02_f32).powi(15),
        };
        if impact <= 0.0 {
            return 0.0;
        }
        let exploitability =
            8.22 * self.av.value() * self.ac.value() * self.pr.value(self.s) * self.ui.value();
        let base = match self.s {
            Scope::Unchanged => f32::min(impact + exploitability, 10.0),
            Scope::Changed => f32::min(1.08 * (impact + exploitability), 10.0),
        };
        (base * 10.0).ceil() / 10.0
    }

    pub fn severity(&self) -> &'static str {
        match self.base_score() {
            s if s == 0.0 => "None",
            s if s < 4.0 => "Low",
            s if s < 7.0 => "Medium",
            s if s < 9.0 => "High",
            _ => "Critical",
        }
    }
}

#[tauri::command]
pub fn calculate_cvss_base(vector: CvssV3) -> CvssScore {
    CvssScore {
        base_score: vector.base_score(),
        severity: vector.severity().to_string(),
    }
}
