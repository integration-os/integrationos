import axios from "axios";
import qs from "qs";
import { DataObject, OAuthResponse } from "../../lib/types";

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
  try {
    const {
      clientId,
      clientSecret,
      metadata: { code },
    } = body;

    const baseUrl = `https://app.attio.com/oauth`;

    const requestBody = {
      grant_type: "authorization_code",
      code: code,
      client_id: clientId,
      client_secret: clientSecret,
    };

    const response = await axios({
      url: `${baseUrl}/token`,
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
        Accept: "application/json",
      },
      data: qs.stringify(requestBody),
    });

    const {
      data: { access_token: accessToken, token_type: tokenType },
    } = response;

    return {
      accessToken,
      refreshToken: accessToken,
      expiresIn: 2147483647,
      tokenType,
      meta: {},
    };
  } catch (error) {
    throw new Error(`Error fetching access token for Attio: ${error}`);
  }
};
