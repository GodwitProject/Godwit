import { Button } from '../ui/Button';
import { Input } from '../ui/Input';
import { Select } from '../ui/Select';
import type { LogFilters } from '../../lib/logs';

export interface LogFiltersProps {
  filters: LogFilters;
  models: string[];
  onChange: (filters: LogFilters) => void;
  onApply: () => void;
  onClear: () => void;
}

export function LogFilters({ filters, models, onChange, onApply, onClear }: LogFiltersProps) {
  return (
    <div className="bg-surface-container-lowest hairline-border rounded-xl p-container-padding space-y-4">
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Input
          label="Search"
          placeholder="Request ID or model"
          value={filters.search || ''}
          onChange={(e) => onChange({ ...filters, search: e.target.value })}
        />
        <Select
          label="Model"
          value={filters.model || ''}
          onChange={(e) => onChange({ ...filters, model: e.target.value })}
        >
          <option value="">All models</option>
          {models.map((m) => (
            <option key={m} value={m}>{m}</option>
          ))}
        </Select>
        <Select
          label="Status"
          value={filters.status != null ? String(filters.status) : ''}
          onChange={(e) =>
            onChange({ ...filters, status: e.target.value === '' ? undefined : e.target.value })
          }
        >
          <option value="">All statuses</option>
          <option value="200">200 Success</option>
          <option value="429">429 Ratelimit</option>
          <option value="500">500 Error</option>
        </Select>
        <div className="grid grid-cols-2 gap-3">
          <Input
            label="From"
            type="date"
            value={filters.dateFrom || ''}
            onChange={(e) => onChange({ ...filters, dateFrom: e.target.value || undefined })}
          />
          <Input
            label="To"
            type="date"
            value={filters.dateTo || ''}
            onChange={(e) => onChange({ ...filters, dateTo: e.target.value || undefined })}
          />
        </div>
      </div>
      <div className="flex items-center gap-3">
        <Button size="sm" onClick={onApply}>Apply</Button>
        <Button variant="secondary" size="sm" onClick={onClear}>Clear</Button>
      </div>
    </div>
  );
}
