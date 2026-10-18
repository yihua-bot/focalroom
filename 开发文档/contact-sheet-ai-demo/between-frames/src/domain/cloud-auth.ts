export interface CloudStatus { authenticated: boolean; waiting: boolean; origin: string; user: { id: string; email: string; name: string; avatar?: string | null } | null; error: string | null }
export interface CloudAuthRepository {
  status(): Promise<CloudStatus>;
  open(): Promise<void>;
  cancel(): Promise<void>;
  logout(): Promise<boolean>;
  subscribe(callback: () => void): Promise<() => void>;
}
