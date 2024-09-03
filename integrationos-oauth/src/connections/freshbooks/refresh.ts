import axios from 'axios';

import { DataObject, OAuthResponse } from '../../lib/types';

export const refresh = async ({ body }: DataObject): Promise<OAuthResponse> => {
    try {
        const {
            OAUTH_CLIENT_ID: client_id,
            OAUTH_CLIENT_SECRET: client_secret,
            OAUTH_REFRESH_TOKEN: refresh_token,
            OAUTH_REQUEST_PAYLOAD: { redirectUri: redirect_uri },
        } = body;

        const requestBody = {
            grant_type: 'refresh_token',
            client_id,
            refresh_token,
            client_secret,
            redirect_uri,
        };

        const response = await axios.post(
            `https://api.freshbooks.com/auth/oauth/token`,
            requestBody,
        );

        const {
            access_token: accessToken,
            refresh_token: refreshToken,
            expires_in: expiresIn,
            token_type: tokenType,
        } = response.data;

        // Get Additional Information required by hitting me URL
        const additionalData = await axios.get(
            'https://api.freshbooks.com/auth/api/v1/users/me',
            {
                headers: {
                    Authorization: `${tokenType} ${accessToken}`,
                },
            },
        );

        if (!additionalData?.data) {
            throw new Error(`Access token validation failed.`);
        }

        const {
            business: { business_uuid: businessId, account_id: accountId },
        } = additionalData.data.response.business_memberships[0];

        return {
            accessToken,
            refreshToken,
            expiresIn,
            tokenType,
            meta: {
                businessId,
                accountId,
            },
        };
    } catch (error) {
        throw new Error(`Error fetching access token for Freshbooks: ${error}`);
    }
};
