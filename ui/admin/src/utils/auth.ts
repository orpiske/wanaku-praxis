const OIDC_PROXY_BASE = '/q/oidc';
const TOKEN_KEY = 'wanaku_auth_token';
const TOKEN_EXPIRY_KEY = 'wanaku_auth_token_expiry';
const CODE_VERIFIER_KEY = 'wanaku_pkce_verifier';
const AUTH_ENABLED_KEY = 'wanaku_auth_enabled';

let cachedToken: string | null = null;
let cachedExpiry: number = 0;

interface TokenResponse {
  access_token: string;
  expires_in: number;
  token_type: string;
  refresh_token?: string;
}

function base64UrlEncode(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (const b of bytes) {
    binary += String.fromCharCode(b);
  }
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

async function generateCodeVerifier(): Promise<string> {
  const array = new Uint8Array(32);
  crypto.getRandomValues(array);
  return base64UrlEncode(array.buffer);
}

async function generateCodeChallenge(verifier: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(verifier);
  const digest = await crypto.subtle.digest('SHA-256', data);
  return base64UrlEncode(digest);
}

export async function isAuthEnabled(): Promise<boolean> {
  const cached = sessionStorage.getItem(AUTH_ENABLED_KEY);
  if (cached !== null) {
    return cached === 'true';
  }

  try {
    const resp = await fetch(`${window.location.origin}/healthz`);
    if (resp.ok) {
      const data = await resp.json();
      const enabled = data?.data?.auth !== 'disabled';
      sessionStorage.setItem(AUTH_ENABLED_KEY, String(enabled));
      return enabled;
    }
  } catch {
    // If health check fails, assume no auth
  }

  sessionStorage.setItem(AUTH_ENABLED_KEY, 'false');
  return false;
}

export function getAccessToken(): string | null {
  if (cachedToken && Date.now() < cachedExpiry) {
    return cachedToken;
  }

  const token = sessionStorage.getItem(TOKEN_KEY);
  const expiry = Number(sessionStorage.getItem(TOKEN_EXPIRY_KEY) || '0');

  if (token && Date.now() < expiry) {
    cachedToken = token;
    cachedExpiry = expiry;
    return token;
  }

  cachedToken = null;
  cachedExpiry = 0;
  return null;
}

function storeToken(tokenResponse: TokenResponse): void {
  const expiresAt = Date.now() + (tokenResponse.expires_in - 30) * 1000;
  cachedToken = tokenResponse.access_token;
  cachedExpiry = expiresAt;
  sessionStorage.setItem(TOKEN_KEY, tokenResponse.access_token);
  sessionStorage.setItem(TOKEN_EXPIRY_KEY, String(expiresAt));
}

export function clearToken(): void {
  cachedToken = null;
  cachedExpiry = 0;
  sessionStorage.removeItem(TOKEN_KEY);
  sessionStorage.removeItem(TOKEN_EXPIRY_KEY);
  sessionStorage.removeItem(CODE_VERIFIER_KEY);
  sessionStorage.removeItem(AUTH_ENABLED_KEY);
}

export async function startAuthFlow(): Promise<void> {
  const verifier = await generateCodeVerifier();
  const challenge = await generateCodeChallenge(verifier);

  sessionStorage.setItem(CODE_VERIFIER_KEY, verifier);

  const params = new URLSearchParams({
    response_type: 'code',
    client_id: 'wanaku-mcp-router',
    redirect_uri: `${window.location.origin}/admin/`,
    code_challenge: challenge,
    code_challenge_method: 'S256',
    scope: 'openid',
  });

  window.location.href = `${window.location.origin}${OIDC_PROXY_BASE}/authorize?${params.toString()}`;
}

export async function handleAuthCallback(): Promise<boolean> {
  const params = new URLSearchParams(window.location.search);
  const code = params.get('code');

  if (!code) {
    return false;
  }

  const verifier = sessionStorage.getItem(CODE_VERIFIER_KEY);
  if (!verifier) {
    console.error('Missing PKCE code verifier');
    return false;
  }

  try {
    const body = new URLSearchParams({
      grant_type: 'authorization_code',
      client_id: 'wanaku-mcp-router',
      code,
      redirect_uri: `${window.location.origin}/admin/`,
      code_verifier: verifier,
    });

    const resp = await fetch(`${window.location.origin}${OIDC_PROXY_BASE}/token`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: body.toString(),
    });

    if (!resp.ok) {
      console.error('Token exchange failed:', resp.status);
      return false;
    }

    const tokenResponse: TokenResponse = await resp.json();
    storeToken(tokenResponse);
    sessionStorage.removeItem(CODE_VERIFIER_KEY);

    // Clean up the URL by removing the auth code
    const cleanUrl = window.location.pathname + window.location.hash;
    window.history.replaceState({}, '', cleanUrl);

    return true;
  } catch (e) {
    console.error('Token exchange error:', e);
    return false;
  }
}

export async function ensureAuthenticated(): Promise<void> {
  const enabled = await isAuthEnabled();
  if (!enabled) {
    return;
  }

  if (await handleAuthCallback()) {
    return;
  }

  const token = getAccessToken();
  if (!token) {
    await startAuthFlow();
  }
}
