import axios from "axios";

import { DataObject, OAuthResponse } from "../../lib/types";
import { convertToTimestamp } from "../../lib/helpers";

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
    try {
        const requestBody = {
            grant_type: "authorization_code",
            code: body.metadata?.code,
            client_id: body.clientId,
            client_secret: body.clientSecret,
            redirect_uri: body.metadata?.redirectUri,
        };

        const response = await axios.post(`https://connect.squareup.com/oauth2/token`, requestBody);

        const {
            data: {
                access_token,
                refresh_token,
                expires_at,
                token_type,
                merchant_id,
                short_lived
            }
        } = response;

        return {
            accessToken: access_token,
            refreshToken: refresh_token,
            expiresIn: Math.floor((await convertToTimestamp(expires_at) - (new Date().getTime())) / 1000),
            tokenType: token_type === "bearer" ? "Bearer" : token_type,
            meta: {
                merchantId: merchant_id,
                shortLived: short_lived
            }
        };
    } catch (error) {
        throw new Error(`Error fetching access token for Square: ${error}`);
    }
};