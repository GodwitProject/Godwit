import { Card } from './Card';

interface EmptyStateProps {
  title?: string;
  message?: string;
  action?: React.ReactNode;
}

export function EmptyState({
  title = 'Nothing here',
  message = 'No items to display.',
  action,
}: EmptyStateProps) {
  return (
    <Card className="flex flex-col items-center justify-center py-16 text-center">
      <h3 className="text-section-sm text-on-surface">{title}</h3>
      <p className="text-body-base text-on-surface-variant mt-2">{message}</p>
      {action && <div className="mt-6">{action}</div>}
    </Card>
  );
}
