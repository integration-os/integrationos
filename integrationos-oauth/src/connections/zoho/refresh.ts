import axios from 'axios';
import { DataObject, OAuthResponse } from '../../lib/types';

export const refresh = async ({ body }: DataObject): Promise<OAuthResponse> => {
    try {
        const {
            OAUTH_CLIENT_ID: client_id,
            OAUTH_CLIENT_SECRET: client_secret,
            OAUTH_REFRESH_TOKEN: refresh_token,
            OAUTH_METADATA: { meta },
        } = body;

        let refreshToken = refresh_token;
        const ZOHO_ACCOUNTS_DOMAIN = meta.ZOHO_ACCOUNTS_DOMAIN;

        let url = `${ZOHO_ACCOUNTS_DOMAIN}/oauth/v2/token?grant_type=refresh_token`;
        url += `&client_id=${client_id}&client_secret=${client_secret}&refresh_token=${refresh_token}`;

        const response = await axios.post(url);

        const {
            data: {
                access_token: accessToken,
                token_type: tokenType,
                expires_in: expiresIn,
            },
        } = response;

        // Update refresh token if a new token is allocated
        if (response.data.refresh_token) {
            refreshToken = response.data.refresh_token;
        }

        return {
            accessToken,
            refreshToken,
            expiresIn,
            tokenType,
            meta,
        };
    } catch (error) {
        throw new Error(`Error fetching refresh token for Zoho: ${error}`);
    }
};
