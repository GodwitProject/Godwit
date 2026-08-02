#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    SuperAdmin,
    OrgAdmin,
    TeamAdmin,
    User,
}

impl Role {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "super_admin" => Some(Role::SuperAdmin),
            "org_admin" => Some(Role::OrgAdmin),
            "team_admin" => Some(Role::TeamAdmin),
            "user" => Some(Role::User),
            _ => None,
        }
    }

    pub fn can_manage_users(&self) -> bool {
        matches!(self, Role::SuperAdmin | Role::OrgAdmin)
    }

    pub fn can_manage_orgs(&self) -> bool {
        matches!(self, Role::SuperAdmin)
    }

    pub fn can_manage_api_keys(&self) -> bool {
        matches!(self, Role::SuperAdmin | Role::OrgAdmin | Role::TeamAdmin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_permissions() {
        assert!(Role::OrgAdmin.can_manage_users());
        assert!(!Role::User.can_manage_users());
        assert!(Role::SuperAdmin.can_manage_orgs());
        assert!(!Role::OrgAdmin.can_manage_orgs());
    }
}
