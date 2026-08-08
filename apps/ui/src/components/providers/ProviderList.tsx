import { Card } from '../ui/Card';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '../ui/Table';
import { Badge } from '../ui/Badge';
import { useT } from '@/hooks/useT';
import type { Provider } from '../../lib/providers';

export interface ProviderListProps {
  providers: Provider[];
}

function short(name: string): string {
  return name
    .split(/[\s-]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0])
    .join('')
    .toUpperCase()
    .slice(0, 3) || 'PR';
}

export function ProviderList({ providers }: ProviderListProps) {
  const { t } = useT();

  if (providers.length === 0) {
    return (
      <Card>
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <span className="text-3xl text-muted mb-2">🔌</span>
          <p className="text-[13px] text-muted">{t('providers.noProviders')}</p>
        </div>
      </Card>
    );
  }

  return (
    <Card className="overflow-hidden">
      <div className="px-4 py-3 border-b border-border">
        <h3 className="text-[13px] font-semibold">{t('providers.listTitle')}</h3>
      </div>
      <Table>
        <TableHead>
          <TableRow>
            <TableHeadCell>{t('nav.providers')}</TableHeadCell>
            <TableHeadCell>{t('providers.protocol')}</TableHeadCell>
            <TableHeadCell>{t('providers.baseUrl')}</TableHeadCell>
            <TableHeadCell>{t('providers.credentials')}</TableHeadCell>
            <TableHeadCell>{t('providers.status')}</TableHeadCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {providers.map((provider) => (
            <TableRow key={provider.id}>
              <TableCell>
                <div className="flex items-center gap-3">
                  <span
                    className="grid place-items-center w-[30px] h-[30px] rounded-lg font-bold text-xs text-white"
                    style={{ background: 'oklch(58% 0.16 145)' }}
                  >
                    {short(provider.name)}
                  </span>
                  <span className="font-medium">{provider.name}</span>
                </div>
              </TableCell>
              <TableCell className="font-mono text-[11.5px] text-muted">{provider.protocol}</TableCell>
              <TableCell className="font-mono text-[11.5px] text-muted truncate max-w-xs">{provider.base_url}</TableCell>
              <TableCell>
                <Badge variant={provider.has_credentials ? 'success' : 'warning'}>
                  {provider.has_credentials ? t('providers.configured') : t('providers.missing')}
                </Badge>
              </TableCell>
              <TableCell>
                <Badge variant={provider.enabled ? 'success' : 'default'}>
                  {provider.enabled ? t('providers.enabled') : t('providers.disabled')}
                </Badge>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </Card>
  );
}
