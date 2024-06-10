import axios from "axios";

import { DataObject, OAuthResponse } from "../../lib/types";
export const refresh = async ({ body }: DataObject): Promise<OAuthResponse> => {
  try {
    const {
      OAUTH_CLIENT_ID: client_id,
      OAUTH_CLIENT_SECRET: client_secret,
      OAUTH_REFRESH_TOKEN: refresh_token,
    } = body;

    const requestBody = {
      grant_type: "refresh_token",
      client_id,
      client_secret,
      refresh_token,
    };

    const response = await axios.post(
      `https://oauth.accounting.sage.com/token`,
      requestBody
    );

    const {
      data: {
        access_token: accessToken,
        refresh_token: refreshToken,
        expires_in: expiresIn,
        requested_by_id: requestedById,
      },
    } = response;

    return {
      accessToken,
      refreshToken,
      expiresIn,
      tokenType: "",
      meta: {
        requestedById,
      },
    };
  } catch (error) {
    throw new Error(`Error fetching access token for SageAccounting: ${error}`);
  }
};
