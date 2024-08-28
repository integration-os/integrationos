import axios from "axios";
import qs from "qs";
import { DataObject, OAuthResponse } from "../../lib/types";

export const refresh = async ({ body }: DataObject): Promise<OAuthResponse> => {
  try {
    const {
      OAUTH_CLIENT_ID: client_id,
      OAUTH_CLIENT_SECRET: client_secret,
      OAUTH_REFRESH_TOKEN: refresh_token,
      OAUTH_REQUEST_PAYLOAD: { redirectUri: redirect_uri, tokenType, meta },
    } = body;

    const requestBody = {
      grant_type: "refresh_token",
      client_id,
      refresh_token,
      client_secret,
      redirect_uri,
    };

    const response = await axios({
      url: `https://api.hubapi.com/oauth/v1/token`,
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
        Accept: "application/json",
      },
      data: qs.stringify(requestBody),
    });

    const {
      access_token: accessToken,
      refresh_token: refreshToken,
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
    throw new Error(`Error fetching access token for Hubspot: ${error}`);
  }
};
