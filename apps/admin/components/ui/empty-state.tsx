export function EmptyState({
  message = 'No items found',
  action,
}: {
  message?: string
  action?: { label: string; onClick: () => void }
}) {
  return (
    <div className="rounded-lg border-2 border-dashed border-gray-300 bg-gray-50 p-12 text-center">
      <p className="text-gray-600">{message}</p>
      {action && (
        <button
          onClick={action.onClick}
          className="mt-4 rounded bg-blue-600 px-4 py-2 text-white hover:bg-blue-700"
        >
          {action.label}
        </button>
      )}
    </div>
  )
}
