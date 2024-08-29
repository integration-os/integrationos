import axios from "axios";
import { DataObject, OAuthResponse } from "../../lib/types";
import { generateBasicHeaders } from "../../lib/helpers";

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
  try {
    const {
      clientId: client_id,
      clientSecret: client_secret,
      metadata: { code, redirectUri: redirect_uri },
    } = body;

    const requestBody = {
      grant_type: "authorization_code",
      code,
      client_id,
      redirect_uri,
    };

    const response = await axios.post(
      "https://app.gong.io/oauth2/generate-customer-token",
      requestBody,
      {
        headers: generateBasicHeaders(client_id, client_secret),
      }
    );

    const {
      access_token: accessToken,
      refresh_token: refreshToken,
      token_type: tokenType,
      expires_in: expiresIn,
      api_base_url_for_customer: apiBaseUrl,
    } = response.data;

    return {
      accessToken,
      refreshToken,
      expiresIn,
      tokenType,
      meta: {
        apiBaseUrl,
      },
    };
  } catch (error) {
    throw new Error(`Error fetching access token for Gong: ${error}`);
  }
};
