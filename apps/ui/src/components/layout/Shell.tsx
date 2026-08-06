import { Sidebar } from './Sidebar';
import { Header } from './Header';
import { MobileNav } from './MobileNav';

export function Shell({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen pb-20 md:pb-0">
      <Header />
      <Sidebar />
      <main className="pt-20 px-margin-mobile md:px-margin-desktop md:ml-sidebar-width max-w-7xl mx-auto flex flex-col gap-8 pb-12">
        {children}
      </main>
      <MobileNav />
    </div>
  );
}
