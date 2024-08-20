import axios from "axios";
import qs from "qs";
import { DataObject, OAuthResponse } from "../../lib/types";
import { differenceInSeconds, generateBasicHeaders } from "../../lib/helpers";

export const refresh = async ({ body }: DataObject): Promise<OAuthResponse> => {
  try {
    const {
      OAUTH_CLIENT_ID: clientId,
      OAUTH_CLIENT_SECRET: clientSecret,
      OAUTH_REFRESH_TOKEN: refresh_token,
      OAUTH_REQUEST_PAYLOAD: {
        formData: { SALESFORCE_DOMAIN },
      },
      OAUTH_METADATA,
    } = body;
    const baseUrl = `${SALESFORCE_DOMAIN}/services/oauth2`;

    const requestBody = {
      grant_type: "refresh_token",
      refresh_token,
    };
    const response = await axios({
      url: `${baseUrl}/token`,
      method: "POST",
      headers: generateBasicHeaders(clientId, clientSecret),
      data: qs.stringify(requestBody),
    });

    const {
      data: { access_token: accessToken, token_type: tokenType },
    } = response;

    let refreshToken = refresh_token;
    if (response.data.refresh_token) {
      refreshToken = response.data.refresh_token;
    }

    // Get expiry time through introspection
    const introspection = await axios({
      url: `${baseUrl}/introspect`,
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      data: qs.stringify({
        client_id: clientId,
        client_secret: clientSecret,
        token: accessToken,
        token_type_hint: "access_token",
      }),
    });
    const {
      data: { exp: expiresAt },
    } = introspection;
    // Converting expiresAt to date object and getting difference in seconds
    const expiresIn = differenceInSeconds(new Date(expiresAt * 1000));

    return {
      accessToken,
      refreshToken,
      expiresIn,
      tokenType,
      meta: {
        ...OAUTH_METADATA?.meta,
      },
    };
  } catch (error) {
    throw new Error(`Error fetching refresh token for Salesforce: ${error}`);
  }
};
