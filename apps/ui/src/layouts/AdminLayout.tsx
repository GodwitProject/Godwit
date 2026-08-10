import { Outlet } from 'react-router-dom';
import { TopBar } from './TopBar';
import { AdminSidebar } from './AdminSidebar';

export function AdminLayout() {
  return (
    <div className="min-h-screen bg-surface-container-low">
      <TopBar />
      <div className="flex">
        <AdminSidebar />
        <main className="flex-1 p-container-padding max-w-7xl">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
