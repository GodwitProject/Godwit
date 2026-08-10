import { Outlet } from 'react-router-dom';
import { TopBar } from './TopBar';
import { ConsoleSidebar } from './ConsoleSidebar';

export function ConsoleLayout() {
  return (
    <div className="min-h-screen bg-surface-container-low">
      <TopBar />
      <div className="flex">
        <ConsoleSidebar />
        <main className="flex-1 p-container-padding max-w-7xl">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
