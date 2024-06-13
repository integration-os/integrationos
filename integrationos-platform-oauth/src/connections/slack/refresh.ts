import axios from "axios";
import qs from "qs";
import { DataObject, OAuthResponse } from "../../lib/types";

export const refresh = async ({ body }: DataObject): Promise<OAuthResponse> => {
  try {
    return {
      accessToken: body?.OAUTH_METADATA?.accessToken,
      refreshToken: body?.OAUTH_METADATA?.refreshToken,
      expiresIn: body?.OAUTH_METADATA?.expiresIn,
      tokenType: "Bearer",
      meta: body?.OAUTH_METADATA?.meta,
    };
  } catch (error) {
    throw new Error(`Error fetching refresh token for Slack: ${error}`);
  }
};
