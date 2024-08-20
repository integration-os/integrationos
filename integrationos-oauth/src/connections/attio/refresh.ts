import { DataObject, OAuthResponse } from "../../lib/types";

export const refresh = async ({ body }: DataObject): Promise<OAuthResponse> => {
  try {
    const {
      OAUTH_METADATA: { accessToken, refreshToken, tokenType, meta },
    } = body;
    return {
      accessToken,
      refreshToken,
      expiresIn: 2147483647,
      tokenType,
      meta,
    };
  } catch (error) {
    throw new Error(`Error fetching refresh token for Attio: ${error}`);
  }
};
