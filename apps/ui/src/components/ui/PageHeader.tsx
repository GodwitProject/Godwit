interface PageHeaderProps {
  title: string;
  description?: string;
  action?: React.ReactNode;
}

export function PageHeader({ title, description, action }: PageHeaderProps) {
  return (
    <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between border-b hairline-border pb-4 mb-6">
      <div>
        <h1 className="text-headline-md text-on-surface">{title}</h1>
        {description && <p className="text-body-base text-on-surface-variant mt-1">{description}</p>}
      </div>
      {action && <div className="mt-4 sm:mt-0">{action}</div>}
    </div>
  );
}
