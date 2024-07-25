import axios from "axios";

import { DataObject, OAuthResponse } from "../../lib/types";

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
    try {
        const requestBody = {
            grant_type: "authorization_code",
            code: body.metadata?.code,
            client_id: body.clientId,
            client_secret: body.clientSecret,
        };

        const baseUrl = body.metadata?.environment === "live" ? "https://api.clover.com" : "https://sandbox.dev.clover.com";

        const response = await axios.post(`${baseUrl}/oauth/token`, requestBody);

        const accessToken = response.data?.access_token;

        return {
            accessToken,
            refreshToken: accessToken,
            expiresIn: 2147483647,
            tokenType: "Bearer",
            meta: {
                merchantId: body.metadata?.additionalData?.merchant_id,
                employeeId: body.metadata?.additionalData?.employee_id,
                baseUrl
            }
        };
    } catch (error) {
        throw new Error(`Error fetching access token for Clover: ${error}`);
    }
};