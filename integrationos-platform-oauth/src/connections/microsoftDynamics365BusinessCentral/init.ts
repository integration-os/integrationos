import axios from "axios";
import qs from "qs";
import { DataObject, OAuthResponse } from "../../lib/types";

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
  try {
    const requestBody = {
      grant_type: "client_credentials",
      client_id: body.clientId,
      client_secret: body.clientSecret,
      scope: "https://api.businesscentral.dynamics.com/.default",
    };

    const response = await axios({
      url: `https://login.microsoftonline.com/${body.metadata?.formData?.BUSINESS_CENTRAL_TENANT_ID}/oauth2/v2.0/token`,
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      data: qs.stringify(requestBody),
    });

    const {
      data: { access_token, expires_in, token_type },
    } = response;

    return {
      accessToken: access_token,
      refreshToken: "",
      expiresIn: +expires_in,
      tokenType: token_type,
      meta: {},
    };
  } catch (error) {
    throw new Error(
      `Error fetching access token for Microsoft Dynamics 365 Business Central: ${error}`
    );
  }
};
