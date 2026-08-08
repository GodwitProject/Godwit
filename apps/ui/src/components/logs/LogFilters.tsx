import { Button } from '../ui/Button';
import { Input } from '../ui/Input';
import { Select } from '../ui/Select';
import { useT } from '@/hooks/useT';
import type { LogFilters } from '../../lib/logs';

export interface LogFiltersProps {
  filters: LogFilters;
  models: string[];
  onChange: (filters: LogFilters) => void;
  onApply: () => void;
  onClear: () => void;
}

export function LogFilters({ filters, models, onChange, onApply, onClear }: LogFiltersProps) {
  const { t } = useT();
  return (
    <div className="bg-surface border border-border rounded-xl px-4 py-3 flex flex-wrap items-center gap-2">
      <div className="seg">
        <button type="button" className="on">{t('traffic.all')}</button>
        <button type="button">{t('traffic.success')}</button>
        <button type="button">{t('traffic.limited')}</button>
        <button type="button">{t('traffic.errors')}</button>
      </div>
      <div className="w-px h-[22px] bg-border mx-1" />
      <div className="seg">
        <button type="button" className="on">{t('traffic.global')}</button>
        <button type="button">chat</button>
        <button type="button">embed</button>
      </div>
      <div className="ml-auto flex flex-wrap items-center gap-2">
        <Select
          label=""
          value={filters.model || ''}
          onChange={(e) => onChange({ ...filters, model: e.target.value || undefined })}
          className="py-1.5"
        >
          <option value="">{t('logs.filter.allModels')}</option>
          {models.map((m) => (
            <option key={m} value={m}>{m}</option>
          ))}
        </Select>
        <Input
          value={filters.api_key_id || ''}
          onChange={(e) => onChange({ ...filters, api_key_id: e.target.value || undefined })}
          placeholder={t('logs.filter.apiKeyId')}
          className="w-40 py-1.5"
        />
        <Input
          type="date"
          value={filters.from || ''}
          onChange={(e) => onChange({ ...filters, from: e.target.value || undefined })}
          className="py-1.5"
        />
        <Input
          type="date"
          value={filters.to || ''}
          onChange={(e) => onChange({ ...filters, to: e.target.value || undefined })}
          className="py-1.5"
        />
        <Button size="sm" onClick={onApply}>{t('logs.filter.apply')}</Button>
        <Button variant="secondary" size="sm" onClick={onClear}>{t('logs.filter.clear')}</Button>
      </div>
    </div>
  );
}
