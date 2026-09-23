// Client de l'API admin-console, engendré par `rbs generate client --lang ts`.
//
// Régénérez-le après chaque changement de contrat plutôt que de le retoucher : la
// commande refuse d'écraser un fichier modifié, et `--force` lève ce refus.

/**
 * Opérateurs acceptés sur une colonne booléenne.
 * 
 * Les conditions écrites ensemble valent un ET.
 */
export interface BoolComparisonOperators {
  eq?: boolean | null;
  gt?: boolean | null;
  gte?: boolean | null;
  in?: boolean[] | null;
  is_null?: boolean | null;
  lt?: boolean | null;
  lte?: boolean | null;
}

export type BoolComparisonSchema = boolean | BoolComparisonOperators

export interface ChangePasswordRequest {
  current_password: string;
  new_password: string;
}

export interface CreateIncident {
  constate_le: string;
  detail: string;
  duree_minutes?: number | null;
  echeance?: string | null;
  gravite: IncidentGravite;
  ouvert: boolean;
  reference: string;
  sujet: string;
}

/**
 * Opérateurs acceptés sur une colonne de date sans heure.
 * 
 * Les conditions écrites ensemble valent un ET.
 */
export interface DateComparisonOperators {
  eq?: null | DateSchema;
  gt?: null | DateSchema;
  gte?: null | DateSchema;
  in?: DateSchema[] | null;
  is_null?: boolean | null;
  lt?: null | DateSchema;
  lte?: null | DateSchema;
}

export type DateComparisonSchema = DateSchema | DateComparisonOperators

export type DateSchema = string

/**
 * Opérateurs acceptés sur une colonne datée.
 * 
 * Les conditions écrites ensemble valent un ET.
 */
export interface DateTimeComparisonOperators {
  eq?: null | DateTimeSchema;
  gt?: null | DateTimeSchema;
  gte?: null | DateTimeSchema;
  in?: DateTimeSchema[] | null;
  is_null?: boolean | null;
  lt?: null | DateTimeSchema;
  lte?: null | DateTimeSchema;
}

export type DateTimeComparisonSchema = DateTimeSchema | DateTimeComparisonOperators

export type DateTimeSchema = string

/**
 * Ce que postent `forgot-password`, `resend-verification` et `PATCH /auth/me`.
 * 
 * Une seule structure pour les trois : elles prennent la même chose, et trois structures
 * identiques divergeraient un jour sans raison. C'est aussi ce qui tient la règle que la
 * dernière porte — `PATCH /auth/me` n'accepte *que* l'adresse : un champ ajouté ici
 * serait aussitôt visible dans les deux autres, et ne passerait pas inaperçu.
 */
export interface EmailRequest {
  email: string;
}

export interface IncidentFilter {
  constate_le?: null | DateTimeComparisonSchema;
  created_at?: null | DateTimeComparisonSchema;
  detail?: null | TextMatchSchema;
  duree_minutes?: null | IntComparisonSchema;
  echeance?: null | DateComparisonSchema;
  gravite?: null | OneOfSchemaIncidentGravite;
  id?: null | UuidComparisonSchema;
  ouvert?: null | BoolComparisonSchema;
  reference?: null | TextMatchSchema;
  sort?: string[] | null;
  sujet?: null | TextMatchSchema;
  updated_at?: null | DateTimeComparisonSchema;
}

export type IncidentGravite = "basse" | "moyenne" | "haute"

export interface IncidentResponse {
  constate_le: string;
  created_at: string;
  detail: string;
  duree_minutes?: number | null;
  echeance?: string | null;
  gravite: IncidentGravite;
  id: string;
  ouvert: boolean;
  reference: string;
  sujet: string;
  updated_at: string;
}

/**
 * Opérateurs acceptés sur une colonne entière.
 * 
 * Les conditions écrites ensemble valent un ET.
 */
export interface IntComparisonOperators {
  eq?: number | null;
  gt?: number | null;
  gte?: number | null;
  in?: number[] | null;
  is_null?: boolean | null;
  lt?: number | null;
  lte?: number | null;
}

export type IntComparisonSchema = number | IntComparisonOperators

export interface LoginRequest {
  email: string;
  password: string;
}

/** Description de la page rendue. */
export interface Meta {
  page: number;
  per_page: number;
  total: number;
  total_pages: number;
}

/**
 * Opérateurs acceptés sur une colonne à valeurs énumérées.
 * 
 * Une énumération ne s'ordonne pas : l'appartenance à une liste y remplace les
 * comparaisons d'une colonne ordonnée.
 */
export interface OneOfOperatorsIncidentGravite {
  eq?: "basse" | "moyenne" | "haute";
  in?: ("basse" | "moyenne" | "haute")[] | null;
  is_null?: boolean | null;
}

export type OneOfSchemaIncidentGravite = "basse" | "moyenne" | "haute" | OneOfOperatorsIncidentGravite

/** Une page de résultats et de quoi situer la suivante. */
export interface PageIncidentResponse {
  data: ({ constate_le: string; created_at: string; detail: string; duree_minutes?: number | null; echeance?: string | null; gravite: IncidentGravite; id: string; ouvert: boolean; reference: string; sujet: string; updated_at: string })[];
  meta: Meta;
}

/** Une page de résultats et de quoi situer la suivante. */
export interface PageUserSummary {
  data: ({ email: string; id: string })[];
  meta: Meta;
}

/**
 * Corps de réponse RFC 9457. Les champs absents ne sont pas sérialisés.
 * 
 * Ce type décrit le corps d'erreur *et* le produit : les deux ne peuvent donc pas
 * diverger, ce qui arriverait avec un schéma OpenAPI rédigé à côté du code.
 */
export interface ProblemDetails {
  detail?: string | null;
  errors?: Record<string, string[]> | null;
  request_id?: string | null;
  status: number;
  title: string;
  type: string;
}

export interface RefreshRequest {
  refresh_token: string;
}

export interface RegisterRequest {
  email: string;
  password: string;
}

/**
 * Ce que rend `GET /auth/registration`.
 * 
 * Le seul moyen qu'a une application servie en fichiers statiques de connaître un
 * réglage que le serveur lit à son démarrage : sans cette route, l'écran d'inscription
 * ne saurait pas qu'il est fermé, et le visiteur ne l'apprendrait qu'en postant.
 */
export interface RegistrationStatus {
  enabled: boolean;
}

export interface ResetPasswordRequest {
  new_password: string;
  token: string;
}

/**
 * La vue publique d'une session.
 * 
 * Jamais `token_hash` : la vue d'une session n'a aucune raison de porter de quoi la
 * présenter.
 */
export interface SessionResponse {
  created_at: string;
  expires_at: string;
  id: string;
}

/**
 * Opérateurs acceptés sur une colonne textuelle.
 * 
 * Un texte se cherche par sous-chaîne et ne s'ordonne pas : ses opérateurs ne sont donc
 * pas ceux d'une colonne comparable.
 */
export interface TextMatchOperators {
  contains?: string | null;
  eq?: string | null;
  is_null?: boolean | null;
}

export type TextMatchSchema = string | TextMatchOperators

/**
 * Ce que rendent `login` et `refresh`.
 * 
 * `refresh_token` est le jeton en clair, remis une seule fois : la base n'en garde que
 * l'empreinte.
 */
export interface TokenPair {
  access_token: string;
  expires_in: number;
  refresh_token: string;
  token_type: string;
}

export interface TokenRequest {
  token: string;
}

export interface UpdateIncident {
  constate_le?: string | null;
  detail?: string | null;
  duree_minutes?: number | null;
  echeance?: string | null;
  gravite?: null | IncidentGravite;
  ouvert?: boolean | null;
  reference?: string | null;
  sujet?: string | null;
}

/** Les conditions de `POST /users/filter`, écrites comme celles d'un filtre engendré. */
export interface UserFilter {
  email?: null | TextMatchSchema;
  id?: null | UuidComparisonSchema;
  sort?: string[] | null;
}

export interface UserResponse {
  created_at: string;
  email: string;
  email_verified_at?: string | null;
  id: string;
  role: string;
}

/**
 * Ce que `POST /users/filter` rend d'un compte : de quoi le choisir et le nommer, rien de
 * plus — ni le rôle ni les dates ne servent à une liste de sélection.
 */
export interface UserSummary {
  email: string;
  id: string;
}

/**
 * Opérateurs acceptés sur une colonne d'identifiants.
 * 
 * Les conditions écrites ensemble valent un ET.
 */
export interface UuidComparisonOperators {
  eq?: string | null;
  gt?: string | null;
  gte?: string | null;
  in?: string[] | null;
  is_null?: boolean | null;
  lt?: string | null;
  lte?: string | null;
}

export type UuidComparisonSchema = string | UuidComparisonOperators

export interface IncidentsListQuery {
  page?: number;
  per_page?: number;
}

export interface IncidentsFilterQuery {
  page?: number;
  per_page?: number;
}

export interface UsersFilterQuery {
  page?: number;
  per_page?: number;
}

/** Ce que le client jette sur une réponse hors 2xx. */
export class ApiError extends Error {
  readonly status: number;
  readonly body: unknown;
  readonly problem?: ProblemDetails;

  constructor(status: number, body: unknown) {
    const problem = isProblem(body) ? body : undefined;
    super(problem?.title ?? `HTTP ${status}`);
    this.name = "ApiError";
    this.status = status;
    this.body = body;
    this.problem = problem;
  }
}

function isProblem(body: unknown): body is ProblemDetails {
  return (
    typeof body === "object" &&
    body !== null &&
    "title" in body &&
    "status" in body
  );
}

/** En-têtes de chaque requête. Une fonction pour un jeton qui tourne. */
export type Headers =
  | Record<string, string>
  | (() => Record<string, string> | Promise<Record<string, string>>);

export interface ApiClientOptions {
  /** Racine de l'API : `https://api.exemple.fr`, ou `/api` sur le même domaine. */
  baseUrl: string;
  /** En-têtes posés sur chaque requête. C'est ici que va le jeton. */
  headers?: Headers;
  /** `fetch` à employer. `globalThis.fetch` par défaut. */
  fetch?: typeof globalThis.fetch;
}

export class ApiClient {
  private readonly baseUrl: string;
  private readonly headers: Headers;
  private readonly fetchImpl: typeof globalThis.fetch;

  constructor(options: ApiClientOptions) {
    // La barre finale est retirée ici plutôt qu'à chaque appel : les chemins du
    // document commencent tous par une barre.
    this.baseUrl = options.baseUrl.replace(/\/+$/, "");
    this.headers = options.headers ?? {};
    this.fetchImpl = options.fetch ?? globalThis.fetch.bind(globalThis);
  }

  /**
   * POST /auth/change-password
   * requiert un jeton
   */
  authChangePassword(body: ChangePasswordRequest): Promise<TokenPair> {
    return this.request<TokenPair>("POST", "/auth/change-password", {
      body,
    });
  }

  /** POST /auth/forgot-password */
  authForgotPassword(body: EmailRequest): Promise<void> {
    return this.request<void>("POST", "/auth/forgot-password", {
      body,
    });
  }

  /** POST /auth/login */
  authLogin(body: LoginRequest): Promise<TokenPair> {
    return this.request<TokenPair>("POST", "/auth/login", {
      body,
    });
  }

  /** POST /auth/logout */
  authLogout(body: RefreshRequest): Promise<void> {
    return this.request<void>("POST", "/auth/logout", {
      body,
    });
  }

  /**
   * GET /auth/me
   * requiert un jeton
   */
  authMe(): Promise<UserResponse> {
    return this.request<UserResponse>("GET", "/auth/me");
  }

  /**
   * PATCH /auth/me
   * requiert un jeton
   */
  authUpdateMe(body: EmailRequest): Promise<void> {
    return this.request<void>("PATCH", "/auth/me", {
      body,
    });
  }

  /** POST /auth/refresh */
  authRefresh(body: RefreshRequest): Promise<TokenPair> {
    return this.request<TokenPair>("POST", "/auth/refresh", {
      body,
    });
  }

  /** POST /auth/register */
  authRegister(body: RegisterRequest): Promise<void> {
    return this.request<void>("POST", "/auth/register", {
      body,
    });
  }

  /** GET /auth/registration */
  authRegistrationStatus(): Promise<RegistrationStatus> {
    return this.request<RegistrationStatus>("GET", "/auth/registration");
  }

  /** POST /auth/resend-verification */
  authResendVerification(body: EmailRequest): Promise<void> {
    return this.request<void>("POST", "/auth/resend-verification", {
      body,
    });
  }

  /** POST /auth/reset-password */
  authResetPassword(body: ResetPasswordRequest): Promise<void> {
    return this.request<void>("POST", "/auth/reset-password", {
      body,
    });
  }

  /**
   * GET /auth/sessions
   * requiert un jeton
   */
  authListSessions(): Promise<SessionResponse[]> {
    return this.request<SessionResponse[]>("GET", "/auth/sessions");
  }

  /**
   * DELETE /auth/sessions
   * requiert un jeton
   */
  authRevokeSessions(): Promise<void> {
    return this.request<void>("DELETE", "/auth/sessions");
  }

  /**
   * DELETE /auth/sessions/{id}
   * requiert un jeton
   */
  authRevokeSession(id: string): Promise<void> {
    return this.request<void>("DELETE", `/auth/sessions/${encodeURIComponent(String(id))}`);
  }

  /** POST /auth/verify-email */
  authVerifyEmail(body: TokenRequest): Promise<void> {
    return this.request<void>("POST", "/auth/verify-email", {
      body,
    });
  }

  /** GET /health */
  health(): Promise<void> {
    return this.request<void>("GET", "/health");
  }

  /** GET /health/live */
  healthLive(): Promise<void> {
    return this.request<void>("GET", "/health/live");
  }

  /**
   * GET /incidents
   * requiert un jeton
   */
  incidentsList(query: IncidentsListQuery = {}): Promise<PageIncidentResponse> {
    return this.request<PageIncidentResponse>("GET", "/incidents", {
      query,
    });
  }

  /**
   * POST /incidents
   * requiert un jeton
   */
  incidentsCreate(body: CreateIncident): Promise<IncidentResponse> {
    return this.request<IncidentResponse>("POST", "/incidents", {
      body,
    });
  }

  /**
   * Filtrer est une lecture : le corps porte les conditions, que l'URL rendrait illisibles.
   * POST /incidents/filter
   * requiert un jeton
   */
  incidentsFilter(body: IncidentFilter, query: IncidentsFilterQuery = {}): Promise<PageIncidentResponse> {
    return this.request<PageIncidentResponse>("POST", "/incidents/filter", {
      body,
      query,
    });
  }

  /**
   * GET /incidents/{id}
   * requiert un jeton
   */
  incidentsFind(id: string): Promise<IncidentResponse> {
    return this.request<IncidentResponse>("GET", `/incidents/${encodeURIComponent(String(id))}`);
  }

  /**
   * DELETE /incidents/{id}
   * requiert un jeton
   */
  incidentsDelete(id: string): Promise<void> {
    return this.request<void>("DELETE", `/incidents/${encodeURIComponent(String(id))}`);
  }

  /**
   * PATCH /incidents/{id}
   * requiert un jeton
   */
  incidentsUpdate(id: string, body: UpdateIncident): Promise<IncidentResponse> {
    return this.request<IncidentResponse>("PATCH", `/incidents/${encodeURIComponent(String(id))}`, {
      body,
    });
  }

  /**
   * POST /users/filter
   * requiert un jeton
   */
  usersFilter(body: UserFilter, query: UsersFilterQuery = {}): Promise<PageUserSummary> {
    return this.request<PageUserSummary>("POST", "/users/filter", {
      body,
      query,
    });
  }

  private async request<T>(
    method: string,
    path: string,
    // `object` et non `Record<string, unknown>` : une interface de query est fermée, et
    // TypeScript refuse de l'assigner à un `Record` faute d'index signature. Lui en poser
    // une la rendrait ouverte, et une clé mal orthographiée passerait sans un mot.
    options: { query?: object; body?: unknown } = {},
  ): Promise<T> {
    const search = new URLSearchParams();
    for (const [cle, valeur] of Object.entries(options.query ?? {})) {
      if (valeur !== undefined && valeur !== null) {
        search.set(cle, String(valeur));
      }
    }

    // Concaténation, et non `new URL` : une racine relative est le cas normal d'une
    // application servie depuis son propre domaine, et `new URL("/api")` jette.
    const queryString = search.toString();
    const url = `${this.baseUrl}${path}${queryString ? `?${queryString}` : ""}`;

    const headers: Record<string, string> = { accept: "application/json" };
    Object.assign(
      headers,
      typeof this.headers === "function" ? await this.headers() : this.headers,
    );
    if (options.body !== undefined) {
      headers["content-type"] = "application/json";
    }

    const response = await this.fetchImpl(url, {
      method,
      headers,
      body: options.body === undefined ? undefined : JSON.stringify(options.body),
    });

    const payload = await parse(response);

    if (!response.ok) {
      throw new ApiError(response.status, payload);
    }

    return payload as T;
  }
}

async function parse(response: Response): Promise<unknown> {
  if (response.status === 204) {
    return undefined;
  }

  const type = response.headers.get("content-type") ?? "";
  if (type.includes("json")) {
    // Un corps annoncé JSON mais vide ne doit pas masquer le statut réel.
    const texte = await response.text();
    return texte.length === 0 ? undefined : JSON.parse(texte);
  }

  const texte = await response.text();
  return texte.length === 0 ? undefined : texte;
}
