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

export interface ProviderProfile {
  id: string;
  name: string;
  protocol: string;
  base_url: string | null;
  allow_wildcard: boolean;
  enabled: boolean;
  has_credentials: boolean;
  created_at: string;
}

export interface Model {
  id: string;
  public_id: string;
  provider: string;
  provider_profile_id: string;
  provider_model_id: string;
  capabilities: string[];
  pricing: {
    input_price_per_million: number;
    output_price_per_million: number;
  };
  config: Record<string, unknown>;
  created_at: string;
}
