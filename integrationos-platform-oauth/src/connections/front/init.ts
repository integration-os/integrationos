import axios from "axios";
import { DataObject, OAuthResponse } from "../../lib/types";
import { differenceInSeconds } from "../../lib/helpers";

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
      "https://app.frontapp.com/oauth/token",
      requestBody,
      {
        headers: generateHeaders(body.clientId, body.clientSecret),
      }
    );

    const {
      access_token: accessToken,
      refresh_token: refreshToken,
      expires_at: expiresAt,
      token_type: tokenType,
    } = response.data;

    return {
      accessToken,
      refreshToken,
      // Converting expiresAt to date object and getting difference in seconds
      expiresIn: differenceInSeconds(new Date(expiresAt * 1000)),
      tokenType,
      meta: {},
    };
  } catch (error) {
    throw new Error(`Error fetching access token for Front: ${error}`);
  }
};
