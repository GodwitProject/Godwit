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
        <Select
          label="Model"
          value={filters.model || ''}
          onChange={(e) => onChange({ ...filters, model: e.target.value || undefined })}
        >
          <option value="">All models</option>
          {models.map((m) => (
            <option key={m} value={m}>{m}</option>
          ))}
        </Select>
        <Input
          label="API Key ID"
          value={filters.api_key_id || ''}
          onChange={(e) => onChange({ ...filters, api_key_id: e.target.value || undefined })}
          placeholder="Filter by API key"
        />
        <Input
          label="From"
          type="date"
          value={filters.from || ''}
          onChange={(e) => onChange({ ...filters, from: e.target.value || undefined })}
        />
        <Input
          label="To"
          type="date"
          value={filters.to || ''}
          onChange={(e) => onChange({ ...filters, to: e.target.value || undefined })}
        />
      </div>
      <div className="flex items-center gap-3">
        <Button size="sm" onClick={onApply}>Apply</Button>
        <Button variant="secondary" size="sm" onClick={onClear}>Clear</Button>
      </div>
    </div>
  );
}
