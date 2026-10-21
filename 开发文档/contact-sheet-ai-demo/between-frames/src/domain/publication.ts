export interface PublicationInfo {
  id: string; projectId: string; pubType: string; title: string; state: string; createdAt: string;
}
export interface PublicationRepository {
  list(projectId: string): Promise<PublicationInfo[]>;
  create(projectId: string, title: string, pubType?: string): Promise<PublicationInfo>;
  generate(projectId: string, publicationId: string, photosPerPage?: number): Promise<number>;
  pages(projectId: string, publicationId: string): Promise<PublicationPage[]>;
}
export interface PublicationPage {
  id: string; pageNumber: number; widthMm: number; heightMm: number;
  frames: Array<{
    id: string; xMm: number; yMm: number; widthMm: number; heightMm: number;
    fitMode: 'fit' | 'fill' | 'stretch'; thumbnailSrc?: string;
  }>;
}
