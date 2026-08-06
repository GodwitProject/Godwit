export interface User {
  id: string
  email: string
  name?: string
  role: 'super_admin' | 'org_admin' | 'team_admin' | 'user'
  organization_id?: string
  created_at: string
}

export interface Claims {
  sub: string
  user_id: string
  organization_id?: string
  role: string
  exp: number
  iat: number
}

export interface Model {
  id: string
  public_id: string
  provider: string
  provider_profile_id: string
  provider_model_id: string
  capabilities: string[]
  pricing: Record<string, unknown>
  config: Record<string, unknown>
  created_at: string
}

export interface ApiKey {
  id: string
  user_id: string
  team_id: string | null
  organization_id: string
  name: string
  key_prefix: string
  scopes: string[]
  allowed_models: string[]
  budget_limit_usd: string | null
  budget_spent_usd: string
  rate_limit_requests_per_minute: number | null
  expires_at: string | null
  disabled: boolean
  created_at: string
}
