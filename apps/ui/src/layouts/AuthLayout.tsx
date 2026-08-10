import { Outlet } from 'react-router-dom';

export function AuthLayout() {
  return (
    <div className="min-h-screen bg-surface-container-low">
      <Outlet />
    </div>
  );
}
