export interface DataObject {
    [key: string]: any;
}

export interface OAuthResponse {
    accessToken: string;
    refreshToken: string;
    expiresIn: number;
    tokenType: string;
    meta: DataObject;
}
