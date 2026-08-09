export type Lang = 'fr' | 'en';

export type TranslationKey =
  | 'nav.overview'
  | 'nav.traffic'
  | 'nav.models'
  | 'nav.keys'
  | 'nav.settings'
  | 'nav.explorer'
  | 'nav.system'
  | 'nav.logs'
  | 'nav.providers'
  | 'brand.env'
  | 'user.role'
  | 'top.searchPlaceholder'
  | 'top.newRequest'
  | 'top.newKey'
  | 'top.notifications'
  | 'top.shortcuts'
  | 'page.overview.title'
  | 'page.overview.subtitle'
  | 'page.traffic.title'
  | 'page.traffic.subtitle'
  | 'page.models.title'
  | 'page.models.subtitle'
  | 'page.keys.title'
  | 'page.keys.subtitle'
  | 'page.providers.title'
  | 'page.providers.subtitle'
  | 'page.logs.title'
  | 'page.logs.subtitle'
  | 'kpi.totalRequests'
  | 'kpi.latencyP95'
  | 'kpi.tokensProcessed'
  | 'kpi.cost'
  | 'kpi.vsYesterday'
  | 'kpi.peak'
  | 'kpi.requestsTotal'
  | 'kpi.tokens'
  | 'kpi.activeRequests'
  | 'kpi.totalCost'
  | 'kpi.noLiveData'
  | 'accounts.title'
  | 'accounts.organizations'
  | 'accounts.teams'
  | 'accounts.users'
  | 'accounts.apiKeys'
  | 'liveMetrics.title'
  | 'chart.requestsPerMinute'
  | 'chart.requests'
  | 'chart.cacheMiss'
  | 'chart.providerSplit'
  | 'chart.spend'
  | 'recent.title'
  | 'recent.viewAll'
  | 'recent.errors'
  | 'recent.timestamp'
  | 'recent.model'
  | 'recent.error'
  | 'recent.status'
  | 'traffic.exportCsv'
  | 'traffic.newRequest'
  | 'traffic.all'
  | 'traffic.success'
  | 'traffic.limited'
  | 'traffic.errors'
  | 'traffic.global'
  | 'traffic.request'
  | 'traffic.provider'
  | 'traffic.duration'
  | 'traffic.tokensIn'
  | 'traffic.tokensOut'
  | 'traffic.cost'
  | 'traffic.session'
  | 'traffic.noResults'
  | 'traffic.noResultsHint'
  | 'traffic.resetFilters'
  | 'traffic.live'
  | 'models.declared'
  | 'models.declaredCountUnit'
  | 'models.noModels'
  | 'models.noCapacity'
  | 'models.add'
  | 'models.exposed'
  | 'models.providerSideId'
  | 'models.latency'
  | 'models.maxContext'
  | 'models.capacity'
  | 'models.capacitySub'
  | 'models.providers'
  | 'models.providersCount'
  | 'models.providerCol'
  | 'modelForm.title'
  | 'modelForm.exposed'
  | 'modelForm.providerProfile'
  | 'modelForm.selectProvider'
  | 'modelForm.providerSideId'
  | 'modelForm.capabilities'
  | 'modelForm.inputPrice'
  | 'modelForm.outputPrice'
  | 'modelForm.submit'
  | 'modelForm.submitting'
  | 'modelForm.cancel'
  | 'settings.subtitle'
  | 'settings.locale'
  | 'settings.localeLanguage'
  | 'settings.localeHint'
  | 'settings.session'
  | 'settings.sessionEmail'
  | 'settings.sessionRole'
  | 'keys.new'
  | 'keys.active'
  | 'keys.name'
  | 'keys.prefix'
  | 'keys.scopes'
  | 'keys.spent'
  | 'keys.rateLimit'
  | 'keys.expires'
  | 'keys.state'
  | 'keys.actions'
  | 'keys.delete'
  | 'keys.noKeys'
  | 'keys.create.title'
  | 'keys.create.nameLabel'
  | 'keys.create.scopes'
  | 'keys.create.allowedModels'
  | 'keys.create.rateLimitRpm'
  | 'keys.create.rateLimitTpm'
  | 'keys.create.cancel'
  | 'keys.create.submit'
  | 'keys.created.title'
  | 'keys.created.note'
  | 'keys.created.copy'
  | 'keys.created.copied'
  | 'keys.created.done'
  | 'keys.details.title'
  | 'keys.details.spent'
  | 'keys.details.budget'
  | 'keys.details.rateRpm'
  | 'keys.details.rateTpm'
  | 'keys.details.expires'
  | 'keys.details.created'
  | 'keys.details.organization'
  | 'keys.details.user'
  | 'keys.details.noModels'
  | 'keys.details.spendTrend'
  | 'keys.details.noSpend'
  | 'keys.details.revoke'
  | 'keys.details.unblock'
  | 'keys.details.close'
  | 'keys.active'
  | 'keys.status.active'
  | 'keys.status.revoked'
  | 'keys.unlimited'
  | 'providers.listTitle'
  | 'providers.noProviders'
  | 'providers.protocol'
  | 'providers.baseUrl'
  | 'providers.credentials'
  | 'providers.status'
  | 'providers.configured'
  | 'providers.configure'
  | 'providers.missing'
  | 'providers.enabled'
  | 'providers.disabled'
  | 'providers.activeOf'
  | 'logs.filter.title'
  | 'logs.filter.model'
  | 'logs.filter.allModels'
  | 'logs.filter.apiKeyId'
  | 'logs.filter.from'
  | 'logs.filter.to'
  | 'logs.filter.apply'
  | 'logs.filter.clear'
  | 'logs.table.timestamp'
  | 'logs.table.id'
  | 'logs.table.latency'
  | 'logs.table.showing'
  | 'logs.table.loadMore'
  | 'logs.table.loading'
  | 'logs.noLogs'
  | 'logs.detail.title'
  | 'logs.detail.noSelection'
  | 'logs.detail.model'
  | 'logs.detail.provider'
  | 'logs.detail.capability'
  | 'logs.detail.streamed'
  | 'logs.detail.apiKey'
  | 'logs.detail.tokens'
  | 'logs.detail.details'
  | 'logs.detail.detailsNote'
  | 'logs.liveTail'
  | 'logs.export'
  | 'login.title'
  | 'login.subtitle'
  | 'login.email'
  | 'login.password'
  | 'login.submit'
  | 'login.signingIn'
  | 'login.loading'
  | 'login.forgot'
  | 'auth.signOut'
  | 'auth.forgot.title'
  | 'auth.forgot.subtitle'
  | 'auth.forgot.email'
  | 'auth.forgot.submit'
  | 'auth.forgot.sending'
  | 'auth.forgot.success'
  | 'auth.backToLogin'
  | 'auth.reset.title'
  | 'auth.reset.subtitle'
  | 'auth.reset.newPassword'
  | 'auth.reset.confirm'
  | 'auth.reset.submit'
  | 'auth.reset.submitting'
  | 'auth.changeRequired.title'
  | 'auth.changeRequired.subtitle'
  | 'auth.changeRequired.newPassword'
  | 'auth.changeRequired.confirm'
  | 'auth.changeRequired.submit'
  | 'auth.changeRequired.submitting'
  | 'auth.change.currentPassword'
  | 'auth.change.newPassword'
  | 'auth.change.confirm'
  | 'auth.change.submit'
  | 'auth.change.submitting'
  | 'auth.change.success'
  | 'auth.mismatch'
  | 'shortcuts.title'
  | 'shortcuts.search'
  | 'shortcuts.overview'
  | 'shortcuts.traffic'
  | 'shortcuts.models'
  | 'shortcuts.keys'
  | 'shortcuts.close'
  | 'shortcuts.hint'
  | 'conn.connecting'
  | 'conn.live'
  | 'conn.polling'
  | 'conn.offline'
  | 'state.ok'
  | 'state.limited'
  | 'state.error'
  | 'state.retry'
  | 'loading.loading'
  | 'yes'
  | 'no';

export interface TranslationSet {
  [key: string]: string;
}

const fr: TranslationSet = {
  'nav.overview': 'Vue d’ensemble',
  'nav.traffic': 'Trafic',
  'nav.models': 'Modèles',
  'nav.keys': 'Clés API',
  'nav.settings': 'Paramètres',
  'nav.explorer': 'Explorer',
  'nav.system': 'Système',
  'nav.logs': 'Journal',
  'nav.providers': 'Fournisseurs',
  'brand.env': 'prod · eu-west-1',
  'user.role': 'admin · ingénieur plateforme',
  'top.searchPlaceholder': 'Rechercher requête, modèle, clé…',
  'top.newRequest': 'Nouvelle requête',
  'top.newKey': 'Nouvelle clé',
  'top.notifications': 'Notifications',
  'top.shortcuts': 'Raccourcis clavier (?)',
  'page.overview.title': 'Vue d’ensemble',
  'page.overview.subtitle': 'Activité agrégée du proxy au cours des dernières 24 h — trafic, latence, coûts et erreurs en temps réel.',
  'page.traffic.title': 'Trafic',
  'page.traffic.subtitle': 'Journal détaillé des requêtes routées par le proxy. Cliquez une ligne pour inspecter la charge envoyée et reçue.',
  'page.models.title': 'Modèles',
  'page.models.subtitle': 'Déclarez les modèles accessibles via le proxy et rattachez chacun à son fournisseur : identifiant du fournisseur, noms exposés et disponibilité.',
  'page.keys.title': 'Clés API',
  'page.keys.subtitle': 'Clés de sortie vers les fournisseurs ainsi que clés clients autorisées à appeler le proxy.',
  'page.providers.title': 'Fournisseurs',
  'page.providers.subtitle': 'Configurez les fournisseurs LLM et les chaînes de repli.',
  'page.logs.title': 'Journal des requêtes',
  'page.logs.subtitle': 'Inspectez l’historique des requêtes du proxy.',
  'kpi.totalRequests': 'Requêtes totales',
  'kpi.latencyP95': 'Latence p95',
  'kpi.tokensProcessed': 'Tokens traités / 24 h',
  'kpi.cost': 'Coût estimé / 24 h',
  'kpi.vsYesterday': 'vs la veille',
  'kpi.peak': 'req/s en pic',
  'kpi.requestsTotal': 'Requêtes totales',
  'kpi.tokens': 'Tokens',
  'kpi.activeRequests': 'Requêtes actives',
  'kpi.totalCost': 'Coût total',
  'kpi.noLiveData': 'Aucune donnée temps réel',
  'accounts.title': 'Comptes',
  'accounts.organizations': 'Organisations',
  'accounts.teams': 'Équipes',
  'accounts.users': 'Utilisateurs',
  'accounts.apiKeys': 'Clés API',
  'liveMetrics.title': 'Métriques temps réel',
  'chart.requestsPerMinute': 'Requêtes par minute',
  'chart.requests': 'Requêtes',
  'chart.cacheMiss': 'Échec de cache',
  'chart.providerSplit': 'Répartition par fournisseur',
  'chart.spend': 'Dépenses (30 derniers jours)',
  'recent.title': 'Erreurs récentes',
  'recent.viewAll': 'Voir tout le trafic',
  'recent.errors': 'Erreurs récentes',
  'recent.timestamp': 'Horodatage',
  'recent.model': 'Modèle',
  'recent.error': 'Erreur',
  'recent.status': 'Statut',
  'traffic.exportCsv': 'Exporter CSV',
  'traffic.newRequest': 'Nouvelle requête',
  'traffic.all': 'Toutes',
  'traffic.success': 'Succès',
  'traffic.limited': 'Limité',
  'traffic.errors': 'Erreurs',
  'traffic.global': 'Global',
  'traffic.request': 'Requête',
  'traffic.provider': 'Fournisseur',
  'traffic.duration': 'Durée',
  'traffic.tokensIn': 'tokens in',
  'traffic.tokensOut': 'tokens out',
  'traffic.cost': 'Coût',
  'traffic.session': 'Session',
  'traffic.noResults': 'Aucune requête trouvée',
  'traffic.noResultsHint': 'Aucune requête ne correspond aux filtres actifs. Essayez d’élargir la période ou de changer de statut.',
  'traffic.resetFilters': 'Réinitialiser les filtres',
  'traffic.live': 'en direct',
  'models.declared': 'Modèles déclarés',
  'models.declaredCountUnit': 'déclarés',
  'models.noModels': 'Aucun modèle déclaré.',
  'models.noCapacity': 'Aucune activité récente à afficher.',
  'models.add': 'Déclarer un modèle',
  'models.exposed': 'Modèle exposé',
  'models.providerSideId': 'Identifiant côté fournisseur',
  'models.latency': 'Latence',
  'models.maxContext': 'Contexte max',
  'models.capacity': 'Capacité consommée',
  'models.capacitySub': 'tokens/min par modèle',
  'models.providers': 'Fournisseurs',
  'models.providersCount': 'actifs sur',
  'models.providerCol': 'Fournisseur',
  'modelForm.title': 'Déclarer un modèle',
  'modelForm.exposed': 'Modèle exposé',
  'modelForm.providerProfile': 'Fournisseur',
  'modelForm.selectProvider': 'Choisir un fournisseur',
  'modelForm.providerSideId': 'Identifiant côté fournisseur',
  'modelForm.capabilities': 'Capacités (séparées par des virgules)',
  'modelForm.inputPrice': 'Prix entrée / 1 M',
  'modelForm.outputPrice': 'Prix sortie / 1 M',
  'modelForm.submit': 'Déclarer',
  'modelForm.submitting': 'Déclaration…',
  'modelForm.cancel': 'Annuler',
  'settings.subtitle': 'Personnalisez la console et consultez les informations de votre session.',
  'settings.locale': 'Langue',
  'settings.localeLanguage': 'Langue de l’interface',
  'settings.localeHint': 'La préférence est enregistrée dans votre navigateur.',
  'settings.session': 'Session',
  'settings.sessionEmail': 'E-mail',
  'settings.sessionRole': 'Rôle',
  'keys.new': 'Nouvelle clé',
  'keys.active': 'Clés actives',
  'keys.name': 'Nom',
  'keys.prefix': 'Préfixe',
  'keys.scopes': 'Droits',
  'keys.spent': 'Dépensé',
  'keys.rateLimit': 'Limite débit',
  'keys.expires': 'Expire',
  'keys.state': 'État',
  'keys.actions': 'Actions',
  'keys.delete': 'Supprimer',
  'keys.noKeys': 'Aucune clé API créée pour l’instant.',
  'keys.status.active': 'Active',
  'keys.status.revoked': 'Révoquée',
  'keys.unlimited': 'Illimité',
  'keys.create.title': 'Créer une clé API',
  'keys.create.nameLabel': 'Nom',
  'keys.create.scopes': 'Droits',
  'keys.create.allowedModels': 'Modèles autorisés',
  'keys.create.rateLimitRpm': 'Limite débit RPM (facultatif)',
  'keys.create.rateLimitTpm': 'Limite débit TPM (facultatif)',
  'keys.create.cancel': 'Annuler',
  'keys.create.submit': 'Créer la clé',
  'keys.created.title': 'Clé API créée',
  'keys.created.note': 'Copiez cette clé maintenant. Vous ne la verrez plus.',
  'keys.created.copy': 'Copier la clé',
  'keys.created.copied': 'Copiée',
  'keys.created.done': 'Terminé',
  'keys.details.title': 'Détails de la clé',
  'keys.details.spent': 'Dépensé',
  'keys.details.budget': 'Budget',
  'keys.details.rateRpm': 'Limite débit (RPM)',
  'keys.details.rateTpm': 'Limite débit (TPM)',
  'keys.details.expires': 'Expire',
  'keys.details.created': 'Créée',
  'keys.details.organization': 'Organisation',
  'keys.details.user': 'Utilisateur',
  'keys.details.noModels': 'Aucun modèle restreint (tous autorisés).',
  'keys.details.spendTrend': 'Tendance de dépense',
  'keys.details.noSpend': 'Aucune série de dépenses live pour l’instant.',
  'keys.details.revoke': 'Révoquer',
  'keys.details.unblock': 'Débloquer',
  'keys.details.close': 'Fermer',
  'providers.listTitle': 'Liste des fournisseurs',
  'providers.noProviders': 'Aucun fournisseur configuré.',
  'providers.protocol': 'Protocole',
  'providers.baseUrl': 'URL de base',
  'providers.credentials': 'Identifiants',
  'providers.status': 'État',
  'providers.configured': 'Configuré',
  'providers.configure': 'Configurer',
  'providers.missing': 'Manquant',
  'providers.enabled': 'Activé',
  'providers.disabled': 'Désactivé',
  'providers.activeOf': 'actifs sur',
  'logs.filter.title': 'Filtres',
  'logs.filter.model': 'Modèle',
  'logs.filter.allModels': 'Tous les modèles',
  'logs.filter.apiKeyId': 'ID de clé API',
  'logs.filter.from': 'Du',
  'logs.filter.to': 'Au',
  'logs.filter.apply': 'Appliquer',
  'logs.filter.clear': 'Effacer',
  'logs.table.timestamp': 'Horodatage',
  'logs.table.id': 'ID',
  'logs.table.latency': 'Latence',
  'logs.table.showing': 'Affichage de',
  'logs.table.loadMore': 'Charger plus',
  'logs.table.loading': 'Chargement…',
  'logs.noLogs': 'Aucun journal trouvé.',
  'logs.detail.title': 'Détails de la requête',
  'logs.detail.noSelection': 'Aucune requête sélectionnée.',
  'logs.detail.model': 'Modèle',
  'logs.detail.provider': 'Fournisseur',
  'logs.detail.capability': 'Capacité',
  'logs.detail.streamed': 'Streamé',
  'logs.detail.apiKey': 'Clé API',
  'logs.detail.tokens': 'Tokens (in→out)',
  'logs.detail.details': 'Détails',
  'logs.detail.detailsNote': 'Les charges utiles et les détails de garde-fous ne sont pas encore disponibles depuis l’endpoint des journaux de dépenses.',
  'logs.liveTail': 'Flux live',
  'logs.export': 'Exporter',
  'login.title': 'Connexion à Godwit',
  'login.subtitle': 'Console d’administration LLM',
  'login.email': 'E-mail',
  'login.password': 'Mot de passe',
  'login.submit': 'Se connecter',
  'login.signingIn': 'Connexion…',
  'login.loading': 'Chargement…',
  'login.forgot': 'Mot de passe oublié ?',
  'auth.signOut': 'Se déconnecter',
  'auth.forgot.title': 'Réinitialiser le mot de passe',
  'auth.forgot.subtitle': 'Saisissez votre e-mail pour recevoir un lien de réinitialisation.',
  'auth.forgot.email': 'E-mail',
  'auth.forgot.submit': 'Envoyer le lien',
  'auth.forgot.sending': 'Envoi…',
  'auth.forgot.success': 'Si un compte existe pour cet e-mail, un lien de réinitialisation a été envoyé.',
  'auth.backToLogin': 'Retour à la connexion',
  'auth.reset.title': 'Nouveau mot de passe',
  'auth.reset.subtitle': 'Définissez un nouveau mot de passe pour votre compte.',
  'auth.reset.newPassword': 'Nouveau mot de passe',
  'auth.reset.confirm': 'Confirmer le mot de passe',
  'auth.reset.submit': 'Réinitialiser le mot de passe',
  'auth.reset.submitting': 'Réinitialisation…',
  'auth.changeRequired.title': 'Changement de mot de passe requis',
  'auth.changeRequired.subtitle': 'Vous devez définir un nouveau mot de passe avant de continuer.',
  'auth.changeRequired.newPassword': 'Nouveau mot de passe',
  'auth.changeRequired.confirm': 'Confirmer le mot de passe',
  'auth.changeRequired.submit': 'Mettre à jour le mot de passe',
  'auth.changeRequired.submitting': 'Mise à jour…',
  'auth.change.currentPassword': 'Mot de passe actuel',
  'auth.change.newPassword': 'Nouveau mot de passe',
  'auth.change.confirm': 'Confirmer le mot de passe',
  'auth.change.submit': 'Changer le mot de passe',
  'auth.change.submitting': 'Changement…',
  'auth.change.success': 'Mot de passe mis à jour.',
  'auth.mismatch': 'Les mots de passe ne correspondent pas.',
  'shortcuts.title': 'Raccourcis clavier',
  'shortcuts.search': 'Rechercher une requête',
  'shortcuts.overview': 'Vue d’ensemble',
  'shortcuts.traffic': 'Trafic',
  'shortcuts.models': 'Modèles',
  'shortcuts.keys': 'Clés API',
  'shortcuts.close': 'Fermer (modale / tiroir)',
  'shortcuts.hint': 'Astuce : les raccourcis g+ touche restent actifs sans modificateur — tapez-les à la suite.',
  'conn.connecting': 'Connexion…',
  'conn.live': 'Live',
  'conn.polling': 'Interrogation',
  'conn.offline': 'Hors ligne',
  'state.ok': 'Succès',
  'state.limited': 'Limité',
  'state.error': 'Erreur',
  'state.retry': 'Nouvel essai',
  'loading.loading': 'Chargement',
  'yes': 'Oui',
  'no': 'Non',
};

const en: TranslationSet = {
  'nav.overview': 'Overview',
  'nav.traffic': 'Traffic',
  'nav.models': 'Models',
  'nav.keys': 'API Keys',
  'nav.settings': 'Settings',
  'nav.explorer': 'Explorer',
  'nav.system': 'System',
  'nav.logs': 'Logs',
  'nav.providers': 'Providers',
  'brand.env': 'prod · eu-west-1',
  'user.role': 'admin · platform engineer',
  'top.searchPlaceholder': 'Search request, model, key…',
  'top.newRequest': 'New request',
  'top.newKey': 'New key',
  'top.notifications': 'Notifications',
  'top.shortcuts': 'Keyboard shortcuts (?)',
  'page.overview.title': 'Overview',
  'page.overview.subtitle': 'Aggregated proxy activity over the last 24 h — traffic, latency, cost, and errors in real time.',
  'page.traffic.title': 'Traffic',
  'page.traffic.subtitle': 'Detailed journal of requests routed by the proxy. Click a row to inspect the sent and received payload.',
  'page.models.title': 'Models',
  'page.models.subtitle': 'Declare the models reachable through the proxy and attach each to its provider: provider ID, exposed names, and availability.',
  'page.keys.title': 'API Keys',
  'page.keys.subtitle': 'Upstream provider keys as well as client keys allowed to call the proxy.',
  'page.providers.title': 'Providers',
  'page.providers.subtitle': 'Configure LLM providers and fallback chains.',
  'page.logs.title': 'Request Logs',
  'page.logs.subtitle': 'Inspect proxy request history.',
  'kpi.totalRequests': 'Total requests',
  'kpi.latencyP95': 'p95 latency',
  'kpi.tokensProcessed': 'Tokens processed / 24 h',
  'kpi.cost': 'Estimated cost / 24 h',
  'kpi.vsYesterday': 'vs yesterday',
  'kpi.peak': 'req/s at peak',
  'kpi.requestsTotal': 'Total requests',
  'kpi.tokens': 'Tokens',
  'kpi.activeRequests': 'Active requests',
  'kpi.totalCost': 'Total cost',
  'kpi.noLiveData': 'No live metric data yet',
  'accounts.title': 'Accounts',
  'accounts.organizations': 'Organizations',
  'accounts.teams': 'Teams',
  'accounts.users': 'Users',
  'accounts.apiKeys': 'API keys',
  'liveMetrics.title': 'Live Metrics',
  'chart.requestsPerMinute': 'Requests per minute',
  'chart.requests': 'Requests',
  'chart.cacheMiss': 'Cache miss',
  'chart.providerSplit': 'Provider split',
  'chart.spend': 'Spend (last 30 days)',
  'recent.title': 'Recent errors',
  'recent.viewAll': 'View all traffic',
  'recent.errors': 'Recent errors',
  'recent.timestamp': 'Timestamp',
  'recent.model': 'Model',
  'recent.error': 'Error',
  'recent.status': 'Status',
  'traffic.exportCsv': 'Export CSV',
  'traffic.newRequest': 'New request',
  'traffic.all': 'All',
  'traffic.success': 'Success',
  'traffic.limited': 'Limited',
  'traffic.errors': 'Errors',
  'traffic.global': 'Global',
  'traffic.request': 'Request',
  'traffic.provider': 'Provider',
  'traffic.duration': 'Duration',
  'traffic.tokensIn': 'tokens in',
  'traffic.tokensOut': 'tokens out',
  'traffic.cost': 'Cost',
  'traffic.session': 'Session',
  'traffic.noResults': 'No requests found',
  'traffic.noResultsHint': 'No requests match the active filters. Try widening the time range or changing the status.',
  'traffic.resetFilters': 'Reset filters',
  'traffic.live': 'live',
  'models.declared': 'Declared models',
  'models.declaredCountUnit': 'declared',
  'models.noModels': 'No models declared yet.',
  'models.noCapacity': 'No recent activity to display.',
  'models.add': 'Declare a model',
  'models.exposed': 'Exposed model',
  'models.providerSideId': 'Provider-side ID',
  'models.latency': 'Latency',
  'models.maxContext': 'Max context',
  'models.capacity': 'Consumed capacity',
  'models.capacitySub': 'tokens/min per model',
  'models.providers': 'Providers',
  'models.providersCount': 'active of',
  'models.providerCol': 'Provider',
  'modelForm.title': 'Declare a model',
  'modelForm.exposed': 'Exposed model',
  'modelForm.providerProfile': 'Provider',
  'modelForm.selectProvider': 'Choose a provider',
  'modelForm.providerSideId': 'Provider-side ID',
  'modelForm.capabilities': 'Capabilities (comma separated)',
  'modelForm.inputPrice': 'Input price / 1M',
  'modelForm.outputPrice': 'Output price / 1M',
  'modelForm.submit': 'Declare',
  'modelForm.submitting': 'Declaring…',
  'modelForm.cancel': 'Cancel',
  'settings.subtitle': 'Personalize the console and review your session information.',
  'settings.locale': 'Language',
  'settings.localeLanguage': 'Interface language',
  'settings.localeHint': 'The preference is saved in your browser.',
  'settings.session': 'Session',
  'settings.sessionEmail': 'Email',
  'settings.sessionRole': 'Role',
  'keys.new': 'New key',
  'keys.active': 'Active keys',
  'keys.name': 'Name',
  'keys.prefix': 'Prefix',
  'keys.scopes': 'Scopes',
  'keys.spent': 'Spent',
  'keys.rateLimit': 'Rate limit',
  'keys.expires': 'Expires',
  'keys.state': 'Status',
  'keys.actions': 'Actions',
  'keys.delete': 'Delete',
  'keys.noKeys': 'No API keys created yet.',
  'keys.status.active': 'Active',
  'keys.status.revoked': 'Revoked',
  'keys.unlimited': 'Unlimited',
  'keys.create.title': 'Create API Key',
  'keys.create.nameLabel': 'Name',
  'keys.create.scopes': 'Scopes',
  'keys.create.allowedModels': 'Allowed Models',
  'keys.create.rateLimitRpm': 'Rate Limit RPM (optional)',
  'keys.create.rateLimitTpm': 'Rate Limit TPM (optional)',
  'keys.create.cancel': 'Cancel',
  'keys.create.submit': 'Create Key',
  'keys.created.title': 'API Key created',
  'keys.created.note': 'Copy this key now. You won\'t see it again.',
  'keys.created.copy': 'Copy Key',
  'keys.created.copied': 'Copied',
  'keys.created.done': 'Done',
  'keys.details.title': 'Key details',
  'keys.details.spent': 'Spent',
  'keys.details.budget': 'Budget',
  'keys.details.rateRpm': 'Rate limit (RPM)',
  'keys.details.rateTpm': 'Rate limit (TPM)',
  'keys.details.expires': 'Expires',
  'keys.details.created': 'Created',
  'keys.details.organization': 'Organization',
  'keys.details.user': 'User',
  'keys.details.noModels': 'No models restricted (all allowed).',
  'keys.details.spendTrend': 'Spend trend',
  'keys.details.noSpend': 'No live spend series yet.',
  'keys.details.revoke': 'Revoke',
  'keys.details.unblock': 'Unblock',
  'keys.details.close': 'Close',
  'providers.listTitle': 'Provider List',
  'providers.noProviders': 'No providers configured yet.',
  'providers.protocol': 'Protocol',
  'providers.baseUrl': 'Base URL',
  'providers.credentials': 'Credentials',
  'providers.status': 'Status',
  'providers.configured': 'Configured',
  'providers.configure': 'Configure',
  'providers.missing': 'Missing',
  'providers.enabled': 'Enabled',
  'providers.disabled': 'Disabled',
  'providers.activeOf': 'active of',
  'logs.filter.title': 'Filters',
  'logs.filter.model': 'Model',
  'logs.filter.allModels': 'All models',
  'logs.filter.apiKeyId': 'API Key ID',
  'logs.filter.from': 'From',
  'logs.filter.to': 'To',
  'logs.filter.apply': 'Apply',
  'logs.filter.clear': 'Clear',
  'logs.table.timestamp': 'Timestamp',
  'logs.table.id': 'Log ID',
  'logs.table.latency': 'Latency',
  'logs.table.showing': 'Showing',
  'logs.table.loadMore': 'Load more',
  'logs.table.loading': 'Loading…',
  'logs.noLogs': 'No logs found.',
  'logs.detail.title': 'Request details',
  'logs.detail.noSelection': 'No request selected.',
  'logs.detail.model': 'Model',
  'logs.detail.provider': 'Provider',
  'logs.detail.capability': 'Capability',
  'logs.detail.streamed': 'Streamed',
  'logs.detail.apiKey': 'API Key',
  'logs.detail.tokens': 'Tokens (in→out)',
  'logs.detail.details': 'Details',
  'logs.detail.detailsNote': 'Request/response payloads and guardrail details are not available from the spend logs endpoint yet.',
  'logs.liveTail': 'Live Tail',
  'logs.export': 'Export',
  'login.title': 'Sign in to Godwit',
  'login.subtitle': 'Admin LLM proxy console',
  'login.email': 'Email',
  'login.password': 'Password',
  'login.submit': 'Sign in',
  'login.signingIn': 'Signing in…',
  'login.loading': 'Loading…',
  'login.forgot': 'Forgot password?',
  'auth.signOut': 'Sign out',
  'auth.forgot.title': 'Reset password',
  'auth.forgot.subtitle': 'Enter your email to receive a reset link.',
  'auth.forgot.email': 'Email',
  'auth.forgot.submit': 'Send reset link',
  'auth.forgot.sending': 'Sending…',
  'auth.forgot.success': 'If an account exists for this email, a reset link has been sent.',
  'auth.backToLogin': 'Back to sign in',
  'auth.reset.title': 'New password',
  'auth.reset.subtitle': 'Set a new password for your account.',
  'auth.reset.newPassword': 'New password',
  'auth.reset.confirm': 'Confirm password',
  'auth.reset.submit': 'Reset password',
  'auth.reset.submitting': 'Resetting…',
  'auth.changeRequired.title': 'Password change required',
  'auth.changeRequired.subtitle': 'You must set a new password before continuing.',
  'auth.changeRequired.newPassword': 'New password',
  'auth.changeRequired.confirm': 'Confirm password',
  'auth.changeRequired.submit': 'Update password',
  'auth.changeRequired.submitting': 'Updating…',
  'auth.change.currentPassword': 'Current password',
  'auth.change.newPassword': 'New password',
  'auth.change.confirm': 'Confirm password',
  'auth.change.submit': 'Change password',
  'auth.change.submitting': 'Changing…',
  'auth.change.success': 'Password updated.',
  'auth.mismatch': 'Passwords do not match.',
  'shortcuts.title': 'Keyboard shortcuts',
  'shortcuts.search': 'Search a request',
  'shortcuts.overview': 'Overview',
  'shortcuts.traffic': 'Traffic',
  'shortcuts.models': 'Models',
  'shortcuts.keys': 'API keys',
  'shortcuts.close': 'Close (modal / drawer)',
  'shortcuts.hint': 'Tip: g+key shortcuts stay active without a modifier — type them in sequence.',
  'conn.connecting': 'Connecting…',
  'conn.live': 'Live',
  'conn.polling': 'Polling',
  'conn.offline': 'Offline',
  'state.ok': 'Success',
  'state.limited': 'Limited',
  'state.error': 'Error',
  'state.retry': 'Retry',
  'loading.loading': 'Loading',
  'yes': 'Yes',
  'no': 'No',
};

export const translations: Record<Lang, TranslationSet> = { fr, en };
