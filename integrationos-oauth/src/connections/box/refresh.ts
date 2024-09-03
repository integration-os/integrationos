import axios from "axios";
import qs from "qs";
import { DataObject, OAuthResponse } from "../../lib/types";

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
      client_id,
      client_secret,
    };

    const response = await axios({
      url: "https://api.box.com/oauth2/token",
			method: "POST",
			headers: {
				"Content-Type": "application/x-www-form-urlencoded",
        Accept: 'application/json',
			},
			data: qs.stringify(requestBody),
		});

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
    throw new Error(`Error fetching access token for Box: ${error}`);
  }
};
