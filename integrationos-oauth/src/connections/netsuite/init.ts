import axios from "axios";
import qs from "qs";
import { DataObject, OAuthResponse } from "../../lib/types";
import { generateBasicHeaders } from "../../lib/helpers";

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
  try {
    const {
      clientId: client_id,
      clientSecret: client_secret,
      metadata: {
        code,
        formData: { NETSUITE_ACCOUNT_ID },
        redirectUri: redirect_uri,
      },
    } = body;

    const requestBody = {
      grant_type: "authorization_code",
      code,
      redirect_uri,
    };

    const response = await axios({
      url: `https://${NETSUITE_ACCOUNT_ID}.suitetalk.api.netsuite.com/services/rest/auth/oauth2/v1/token`,
      method: "POST",
      headers: generateBasicHeaders(client_id, client_secret),
      data: qs.stringify(requestBody),
    });

    const {
      data: {
        access_token: accessToken,
        refresh_token: refreshToken,
        expires_in: expiresIn,
        token_type: tokenType,
      },
    } = response;

    return {
      accessToken,
      refreshToken,
      expiresIn: +expiresIn,
      tokenType,
      meta: {},
    };
  } catch (error) {
    throw new Error(`Error fetching access token for Netsuite: ${error}`);
  }
};
