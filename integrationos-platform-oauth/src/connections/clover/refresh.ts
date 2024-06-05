import { OAuthResponse } from "../../lib/types";

export const refresh = async (payload: any): Promise<OAuthResponse> => {
    return {
        accessToken: payload.accessToken,
        refreshToken: payload.refreshToken,
        expiresIn: payload.expiresIn,
        tokenType: payload.tokenType,
        meta: payload.meta
    };
};
