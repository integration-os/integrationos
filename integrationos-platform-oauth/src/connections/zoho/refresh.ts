import axios from "axios";
import { DataObject, OAuthResponse } from "../../lib/types";

export const refresh = async ({ body }: DataObject): Promise<OAuthResponse> => {
  try {
    const {
      OAUTH_CLIENT_ID: client_id,
      OAUTH_CLIENT_SECRET: client_secret,
      OAUTH_REFRESH_TOKEN: refresh_token,
      OAUTH_REQUEST_PAYLOAD: {
        formData: { ZOHO_ACCOUNTS_DOMAIN },
      },
      OAUTH_METADATA,
    } = body;

    let url = `${ZOHO_ACCOUNTS_DOMAIN}/oauth/v2/token?grant_type=refresh_token`;
    url += `&client_id=${client_id}&client_secret=${client_secret}&refresh_token=${refresh_token}`;

    const response = await axios.post(url);

    const {
      data: {
        access_token: accessToken,
        refresh_token: refreshToken,
        token_type: tokenType,
        expires_in: expiresIn,
      },
    } = response;

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
    throw new Error(`Error fetching refresh token for Zoho: ${error}`);
  }
};
