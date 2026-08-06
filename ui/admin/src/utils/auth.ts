import { UserManager } from 'oidc-client-ts';

const OIDC_PROXY_BASE = '/q/oidc';
const DISCOVERY_URL = `${OIDC_PROXY_BASE}/.well-known/openid-configuration`;

let userManager: UserManager | null = null;
let authEnabledChecked = false;
let authEnabledResult = false;

async function getOrCreateUserManager(): Promise<UserManager> {
  if (userManager) {
    return userManager;
  }

  const discoveryResp = await fetch(`${window.location.origin}${DISCOVERY_URL}`);
  if (!discoveryResp.ok) {
    throw new Error(`OIDC discovery failed: ${discoveryResp.status}`);
  }

  const metadata = await discoveryResp.json();

  userManager = new UserManager({
    authority: metadata.issuer,
    client_id: 'wanaku-mcp-router',
    redirect_uri: `${window.location.origin}/admin/`,
    post_logout_redirect_uri: `${window.location.origin}/admin/`,
    response_type: 'code',
    scope: 'openid',
    automaticSilentRenew: false,
    monitorSession: false,
    metadata,
  });

  return userManager;
}

export async function isAuthEnabled(): Promise<boolean> {
  if (authEnabledChecked) {
    return authEnabledResult;
  }

  try {
    const resp = await fetch(`${window.location.origin}/healthz`);
    if (resp.ok) {
      const data = await resp.json();
      const authField = data?.data?.auth;
      authEnabledResult = authField !== undefined && authField !== 'disabled';
      authEnabledChecked = true;
      return authEnabledResult;
    }
  } catch {
    // Don't cache failures — allow retry on next call
  }

  return false;
}

export async function getAccessToken(): Promise<string | null> {
  const enabled = await isAuthEnabled();
  if (!enabled) {
    return null;
  }

  try {
    const mgr = await getOrCreateUserManager();
    const user = await mgr.getUser();

    if (user && !user.expired) {
      return user.access_token;
    }
  } catch {
    // UserManager not initialized yet or user not available
  }

  return null;
}

export function clearAuth(): void {
  authEnabledChecked = false;
  authEnabledResult = false;
  if (userManager) {
    userManager.removeUser();
    userManager = null;
  }
}

export async function startAuthFlow(): Promise<never> {
  const mgr = await getOrCreateUserManager();
  await mgr.signinRedirect();
  return new Promise(() => {});
}

async function handleAuthCallback(): Promise<boolean> {
  const params = new URLSearchParams(window.location.search);
  if (!params.has('code')) {
    return false;
  }

  try {
    const mgr = await getOrCreateUserManager();
    await mgr.signinRedirectCallback(window.location.href);

    const cleanUrl = window.location.pathname + window.location.hash;
    window.history.replaceState({}, '', cleanUrl);

    return true;
  } catch (e) {
    console.error('[wanaku-auth] callback processing failed:', e);
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

  const token = await getAccessToken();
  if (!token) {
    await startAuthFlow();
  }
}
