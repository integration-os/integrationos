import axios from "axios";
import qs from "qs";
import { DataObject, OAuthResponse } from "../../lib/types";

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
      client_secret,
      redirect_uri,
    };

    const response = await axios({
      url: "https://api.dropbox.com/oauth2/token",
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
      account_id: accountId,
      uid: uId
    } = response.data;

    return {
      accessToken,
      refreshToken,
      expiresIn,
      tokenType,
      meta: {
        accountId,
        uId
      },
    };
  } catch (error) {
    throw new Error(`Error fetching access token for Dropbox: ${error}`);
  }
};
