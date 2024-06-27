import axios from "axios";
import { DataObject, OAuthResponse } from "../../lib/types";

const generateHeaders = (clientId: string, clientSecret: string) => {
  const credentials = clientId + ":" + clientSecret;
  const encodedCredentials = Buffer.from(credentials).toString("base64");

  return {
    authorization: "Basic " + encodedCredentials,
    "Content-Type": "application/x-www-form-urlencoded",
  };
};

export const refresh = async ({ body }: DataObject): Promise<OAuthResponse> => {
  try {
    const {
      OAUTH_CLIENT_ID: client_id,
      OAUTH_CLIENT_SECRET: client_secret,
      OAUTH_REFRESH_TOKEN: refresh_token,
    } = body;

    const requestBody = {
      grant_type: "refresh_token",
      refresh_token,
    };

    const response = await axios.post(
      "https://app.frontapp.com/oauth/token",
      requestBody,
      {
        headers: generateHeaders(client_id, client_secret),
      }
    );

    const {
      access_token: accessToken,
      refresh_token: refreshToken,
      expires_at: expiresIn,
      token_type: tokenType,
    } = response.data;

    return {
      accessToken,
      refreshToken,
      // JWT ExpAt value converted to date - expiresIn * 1000
      expiresIn: Math.floor((expiresIn * 1000 - new Date().getTime()) / 1000),
      tokenType,
      meta: {
        ...body?.OAUTH_METADATA?.meta,
      },
    };
  } catch (error) {
    throw new Error(`Error fetching refresh token for front: ${error}`);
  }
};
