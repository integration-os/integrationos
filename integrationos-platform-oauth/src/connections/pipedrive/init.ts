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

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
  try {
    const requestBody = {
      grant_type: "authorization_code",
      code: body.metadata?.code,
      redirect_uri: body.metadata?.redirectUri,
    };

    const response = await axios.post(
      "https://oauth.pipedrive.com/oauth/token",
      requestBody,
      {
        headers: generateHeaders(body.clientId, body.clientSecret),
      }
    );

    const {
      access_token: accessToken,
      refresh_token: refreshToken,
      expires_in: expiresIn,
      token_type: tokenType,
      api_domain: apiDomain,
    } = response.data;

    return {
      accessToken,
      refreshToken,
      expiresIn,
      tokenType,
      meta: {
        apiDomain,
      },
    };
  } catch (error) {
    throw new Error(`Error fetching access token for pipedrive: ${error}`);
  }
};
