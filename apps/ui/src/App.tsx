import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useAuthInit } from '@/hooks/useAuth';
import { RequireRole } from '@/components/auth/RequireRole';
import { AuthLayout } from '@/layouts/AuthLayout';
import { AdminLayout } from '@/layouts/AdminLayout';
import { ConsoleLayout } from '@/layouts/ConsoleLayout';
import { LoginPage } from '@/routes/login';
import { AdminDashboard } from '@/routes/admin';
import { AdminModels } from '@/routes/admin/models';
import { AdminProviderProfiles } from '@/routes/admin/provider-profiles';
import { AdminUsers } from '@/routes/admin/users';
import { AdminKeys } from '@/routes/admin/keys';
import { AdminUsage } from '@/routes/admin/usage';
import { AdminSettings } from '@/routes/admin/settings';
import { ConsoleDashboard } from '@/routes/console';
import { ConsoleModels } from '@/routes/console/models';
import { ConsoleKeys } from '@/routes/console/keys';
import { ConsoleUsage } from '@/routes/console/usage';
import { ConsoleOrganization } from '@/routes/console/organization';
import { ConsoleTeam } from '@/routes/console/team';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

function AppRouter() {
  useAuthInit();

  return (
    <Routes>
      <Route element={<AuthLayout />}>
        <Route path="/login" element={<LoginPage />} />
      </Route>

      <Route
        element={
          <RequireRole allowed={['super_admin']} fallback="/console">
            <AdminLayout />
          </RequireRole>
        }
      >
        <Route path="/admin" element={<AdminDashboard />} />
        <Route path="/admin/models" element={<AdminModels />} />
        <Route path="/admin/provider-profiles" element={<AdminProviderProfiles />} />
        <Route path="/admin/users" element={<AdminUsers />} />
        <Route path="/admin/keys" element={<AdminKeys />} />
        <Route path="/admin/usage" element={<AdminUsage />} />
        <Route path="/admin/settings" element={<AdminSettings />} />
      </Route>

      <Route
        element={
          <RequireRole allowed={['super_admin', 'org_admin', 'team_admin', 'user']}>
            <ConsoleLayout />
          </RequireRole>
        }
      >
        <Route path="/console" element={<ConsoleDashboard />} />
        <Route path="/console/models" element={<ConsoleModels />} />
        <Route path="/console/keys" element={<ConsoleKeys />} />
        <Route path="/console/usage" element={<ConsoleUsage />} />
        <Route path="/console/organization" element={<ConsoleOrganization />} />
        <Route path="/console/team" element={<ConsoleTeam />} />
      </Route>

      <Route path="*" element={<Navigate to="/console" replace />} />
    </Routes>
  );
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <AppRouter />
      </BrowserRouter>
    </QueryClientProvider>
  );
}
