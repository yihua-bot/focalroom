export interface CloudStatus { authenticated: boolean; waiting: boolean; origin: string; user: { id: string; email: string; name: string; avatar?: string | null } | null; error: string | null }
export interface CloudAuthRepository {
  status(): Promise<CloudStatus>;
  open(): Promise<void>;
  cancel(): Promise<void>;
  logout(): Promise<boolean>;
  updateProfile(displayName?: string, avatarUrl?: string): Promise<CloudStatus>;
  subscribe(callback: () => void): Promise<() => void>;
}
