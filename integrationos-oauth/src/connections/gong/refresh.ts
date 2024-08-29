import axios from "axios";
import { DataObject, OAuthResponse } from "../../lib/types";
import { generateBasicHeaders } from "../../lib/helpers";

export const refresh = async ({ body }: DataObject): Promise<OAuthResponse> => {
  try {
    const {
      OAUTH_CLIENT_ID: client_id,
      OAUTH_CLIENT_SECRET: client_secret,
      OAUTH_REFRESH_TOKEN: refresh_token,
      OAUTH_METADATA: { meta },
    } = body;

    const requestBody = {
      grant_type: "refresh_token",
      refresh_token,
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
    } = response.data;

    return {
      accessToken,
      refreshToken,
      expiresIn,
      tokenType,
      meta,
    };
  } catch (error) {
    throw new Error(`Error fetching access token for Gong: ${error}`);
  }
};
