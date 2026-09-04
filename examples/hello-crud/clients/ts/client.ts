// Client de l'API hello-crud, engendré par `rbs generate client --lang ts`.
//
// Régénérez-le après chaque changement de contrat plutôt que de le retoucher : la
// commande refuse d'écraser un fichier modifié, et `--force` lève ce refus.

export interface ArticleFilter {
  body: TextMatchSchema;
  created_at: ComparisonSchema;
  id: ComparisonSchema;
  published: ComparisonSchema;
  sort: string[];
  title: TextMatchSchema;
  updated_at: ComparisonSchema;
}

export interface ArticleResponse {
  body: string;
  created_at: string;
  id: string;
  published: boolean;
  title: string;
  updated_at: string;
}

/**
 * Conditions acceptées sur une colonne comparable.
 * 
 * Une valeur nue, écrite hors de tout objet, vaut la condition `eq`.
 */
export interface ComparisonSchema {
  eq?: unknown;
  gt?: unknown;
  gte?: unknown;
  is_null?: boolean | null;
  lt?: unknown;
  lte?: unknown;
}

export interface CreateArticle {
  body: string;
  published: boolean;
  title: string;
}

/** Description de la page rendue. */
export interface Meta {
  page: number;
  per_page: number;
  total: number;
  total_pages: number;
}

/** Une page de résultats et de quoi situer la suivante. */
export interface PageArticleResponse {
  data: ({ body: string; created_at: string; id: string; published: boolean; title: string; updated_at: string })[];
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

/**
 * Conditions acceptées sur une colonne textuelle.
 * 
 * Une chaîne nue, écrite hors de tout objet, vaut la condition `eq`.
 */
export interface TextMatchSchema {
  contains?: string | null;
  eq?: string | null;
  is_null?: boolean | null;
}

export interface UpdateArticle {
  body?: string | null;
  published?: boolean | null;
  title?: string | null;
}

export interface ArticlesListQuery {
  page?: number;
  per_page?: number;
}

export interface ArticlesFilterQuery {
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

// region: options
export interface ApiClientOptions {
  /** Racine de l'API : `https://api.exemple.fr`, ou `/api` sur le même domaine. */
  baseUrl: string;
  /** En-têtes posés sur chaque requête. C'est ici que va le jeton. */
  headers?: Headers;
  /** `fetch` à employer. `globalThis.fetch` par défaut. */
  fetch?: typeof globalThis.fetch;
}

// endregion: options

// region: classe
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

// endregion: classe

// region: methodes
  /** GET /articles */
  articlesList(query: ArticlesListQuery = {}): Promise<PageArticleResponse> {
    return this.request<PageArticleResponse>("GET", "/articles", {
      query,
    });
  }

  /** POST /articles */
  articlesCreate(body: CreateArticle): Promise<ArticleResponse> {
    return this.request<ArticleResponse>("POST", "/articles", {
      body,
    });
  }

  /**
   * Filtrer est une lecture : le corps porte les conditions, que l'URL rendrait illisibles.
Le garde de rôle ne s'y applique donc pas, pas plus qu'à `list` ou `find`.
   * POST /articles/filter
   */
  articlesFilter(body: ArticleFilter, query: ArticlesFilterQuery = {}): Promise<PageArticleResponse> {
    return this.request<PageArticleResponse>("POST", "/articles/filter", {
      body,
      query,
    });
  }

  /** GET /articles/{id} */
  articlesFind(id: string): Promise<ArticleResponse> {
    return this.request<ArticleResponse>("GET", `/articles/${encodeURIComponent(String(id))}`);
  }

  /** DELETE /articles/{id} */
  articlesDelete(id: string): Promise<void> {
    return this.request<void>("DELETE", `/articles/${encodeURIComponent(String(id))}`);
  }

  /** PATCH /articles/{id} */
  articlesUpdate(id: string, body: UpdateArticle): Promise<ArticleResponse> {
    return this.request<ArticleResponse>("PATCH", `/articles/${encodeURIComponent(String(id))}`, {
      body,
    });
  }

  /** GET /health */
  health(): Promise<void> {
    return this.request<void>("GET", "/health");
  }

// endregion: methodes
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
