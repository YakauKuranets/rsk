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

#[cfg(test)]
mod tests {
    use super::*;

    fn make(
        av: AttackVector,
        ac: AttackComplexity,
        pr: PrivilegesRequired,
        ui: UserInteraction,
        s: Scope,
        c: Impact,
        i: Impact,
        a: Impact,
    ) -> CvssV3 {
        CvssV3 {
            av,
            ac,
            pr,
            ui,
            s,
            c,
            i,
            a,
        }
    }

    #[test]
    fn test_critical_network_no_auth() {
        let v = make(
            AttackVector::Network,
            AttackComplexity::Low,
            PrivilegesRequired::None,
            UserInteraction::None,
            Scope::Unchanged,
            Impact::High,
            Impact::High,
            Impact::High,
        );
        assert_eq!(v.base_score(), 9.8);
        assert_eq!(v.severity(), "Critical");
    }

    #[test]
    fn test_medium_low_priv() {
        let v = make(
            AttackVector::Network,
            AttackComplexity::Low,
            PrivilegesRequired::Low,
            UserInteraction::None,
            Scope::Unchanged,
            Impact::High,
            Impact::None,
            Impact::None,
        );
        assert_eq!(v.base_score(), 6.5);
        assert_eq!(v.severity(), "Medium");
    }

    #[test]
    fn test_low_physical() {
        let v = make(
            AttackVector::Physical,
            AttackComplexity::High,
            PrivilegesRequired::High,
            UserInteraction::Required,
            Scope::Unchanged,
            Impact::None,
            Impact::None,
            Impact::Low,
        );
        let score = v.base_score();
        assert!((1.5..=1.7).contains(&score), "expected ~1.6, got {}", score);
        assert_eq!(v.severity(), "Low");
    }

    #[test]
    fn test_critical_scope_changed() {
        let v = make(
            AttackVector::Network,
            AttackComplexity::Low,
            PrivilegesRequired::None,
            UserInteraction::None,
            Scope::Changed,
            Impact::High,
            Impact::High,
            Impact::High,
        );
        assert_eq!(v.base_score(), 10.0);
    }

    #[test]
    fn test_no_impact_is_zero() {
        let v = make(
            AttackVector::Network,
            AttackComplexity::Low,
            PrivilegesRequired::None,
            UserInteraction::None,
            Scope::Unchanged,
            Impact::None,
            Impact::None,
            Impact::None,
        );
        assert_eq!(v.base_score(), 0.0);
        assert_eq!(v.severity(), "None");
    }

    #[test]
    fn test_calculate_cvss_base_command() {
        let v = make(
            AttackVector::Network,
            AttackComplexity::Low,
            PrivilegesRequired::None,
            UserInteraction::None,
            Scope::Unchanged,
            Impact::High,
            Impact::High,
            Impact::High,
        );
        let result = calculate_cvss_base(v);
        assert_eq!(result.base_score, 9.8);
        assert_eq!(result.severity, "Critical");
    }
}
