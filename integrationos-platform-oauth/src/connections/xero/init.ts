import axios from "axios";
import jwt from "jsonwebtoken";
import { DataObject, OAuthResponse } from "../../lib/types";

const generateXeroHeaders = (clientId: string, clientSecret: string) => {
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
      `https://identity.xero.com/connect/token`,
      requestBody,
      {
        headers: generateXeroHeaders(body.clientId, body.clientSecret),
      }
    );

    const {
      access_token: accessToken,
      refresh_token: refreshToken,
      expires_in: expiresIn,
      token_type: tokenType,
    } = response.data;

    // Get tenant id details
    const decodedToken = jwt.decode(accessToken) as {
      authentication_event_id: string;
    };

    const tenantId = await axios.get("https://api.xero.com/connections", {
      headers: {
        authorization: "Bearer " + accessToken,
        "Content-Type": "application/json",
      },
    });

    if (!tenantId.data.length) {
      throw new Error(`Failed to fetch tenantId from Xero API`);
    }

    const extractedTenantId = tenantId.data.find(
      (tenant: any) =>
        tenant.authEventId === decodedToken.authentication_event_id
    )?.tenantId;

    if (!extractedTenantId) {
      throw new Error(`Failed to extract tenantId from Xero API response`);
    }

    const newMetadata = {
      ...body?.metadata,
      tenantId: extractedTenantId,
    };

    return {
      accessToken,
      refreshToken,
      expiresIn,
      tokenType,
      meta: {
        ...newMetadata,
      },
    };
  } catch (error) {
    throw new Error(`Error fetching access token for xero: ${error}`);
  }
};
