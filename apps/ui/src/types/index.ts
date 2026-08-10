export type UserRole = 'super_admin' | 'org_admin' | 'team_admin' | 'user';

export interface AuthUser {
  id: string;
  email: string;
  role: UserRole;
  organization_id: string | null;
}

export function isRole(user: AuthUser | null, role: UserRole): boolean {
  return user?.role === role;
}
