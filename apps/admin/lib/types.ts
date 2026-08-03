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
