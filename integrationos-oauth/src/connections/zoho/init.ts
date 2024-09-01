import axios from 'axios';
import { DataObject, OAuthResponse } from '../../lib/types';

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
    try {
        const {
            clientId,
            clientSecret,
            metadata: {
                additionalData,
                code,
                formData: { ZOHO_ACCOUNTS_DOMAIN },
                redirectUri,
            },
        } = body;
        let url = `${ZOHO_ACCOUNTS_DOMAIN}/oauth/v2/token?grant_type=authorization_code`;
        url += `&client_id=${clientId}&client_secret=${clientSecret}&code=${code}&redirect_uri=${redirectUri}`;

        const response = await axios.post(url);

        const {
            data: {
                access_token: accessToken,
                refresh_token: refreshToken,
                api_domain: apiDomain,
                token_type: tokenType,
                expires_in: expiresIn,
            },
        } = response;

        return {
            accessToken,
            refreshToken,
            expiresIn,
            tokenType,
            meta: {
                apiDomain,
                ...additionalData,
            },
        };
    } catch (error) {
        throw new Error(`Error fetching access token for Zoho: ${error}`);
    }
};
